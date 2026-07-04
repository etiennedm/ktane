//! Minimal SK6812-RGBW driver built directly on an RMT TX channel.
//!
//! Data is emitted in the SK6812's native G, R, B, W channel order,
//! so [`ColorRGBW`] values are plain logical colors.

use esp_hal::{
    gpio::{interconnect::PeripheralOutput, Level},
    rmt::{Channel, PulseCode, Tx, TxChannelConfig, TxChannelCreator},
    Blocking,
};

/// RMT pulses per LED: SK6812-RGBW carries 32 bits, one pulse each.
const BITS_PER_LED: usize = 32;

/// RMT buffer length to drive `led_count` LEDs: one pulse per bit, plus a
/// trailing end marker to release the line. Use this to compute the `BUF`
/// type parameter of [`SK6812ChainDriver`] from a chain length, e.g.
/// `Leds::<N, { frame_len(N) }>::new(..)`.
pub const fn frame_len(led_count: usize) -> usize {
    led_count * BITS_PER_LED + 1
}

// SK6812 bit timing expressed in RMT ticks at an 80 MHz clock (1 tick = 12.5 ns).
// "0" bit: ~400 ns high then ~850 ns low.
// "1" bit: ~850 ns high then ~400 ns low.
const T0H: u16 = 32;
const T0L: u16 = 68;
const T1H: u16 = 68;
const T1L: u16 = 32;

/// The two RMT symbols for a data bit.
const ZERO: PulseCode = PulseCode::new(Level::High, T0H, Level::Low, T0L);
const ONE: PulseCode = PulseCode::new(Level::High, T1H, Level::Low, T1L);

/// One SK6812-RGBW pixel, in logical channel order.
#[derive(Copy, Clone)]
pub struct ColorRGBW {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub w: u8,
}

impl ColorRGBW {
    pub const OFF: ColorRGBW = ColorRGBW::new(0, 0, 0, 0);

    pub const fn new(r: u8, g: u8, b: u8, w: u8) -> Self {
        Self { r, g, b, w }
    }
}

/// SK6812 driver over a single blocking RMT TX channel, for a chain of
/// `LED_COUNT` LEDs. `BUF` is the RMT buffer length and must be
/// `frame_len(LED_COUNT)`.
pub struct SK6812ChainDriver<'ch, const LED_COUNT: usize, const BUF: usize> {
    rmt_channel: Option<Channel<'ch, Blocking, Tx>>,
    rmt_buffer: [PulseCode; BUF],
}

impl<'ch, const LED_COUNT: usize, const RMT_BUF_SIZE: usize> SK6812ChainDriver<'ch, LED_COUNT, RMT_BUF_SIZE> {
    /// Compile-time check that `BUF` is exactly the buffer length needed for
    /// `LED_COUNT` LEDs. Referenced in `new` to force evaluation.
    const ASSERT_BUF: () = assert!(
        RMT_BUF_SIZE == frame_len(LED_COUNT),
        "Leds BUF parameter must equal frame_len(LED_COUNT)"
    );

    /// Configure the given RMT channel and pin for the SK6812 (idle low, no
    /// carrier) and build the driver.
    pub fn new<C, O>(channel: C, pin: O) -> Self
    where
        C: TxChannelCreator<'ch, Blocking>,
        O: PeripheralOutput<'ch>,
    {
        let () = Self::ASSERT_BUF;

        let config = TxChannelConfig::default()
            .with_clk_divider(1)
            .with_idle_output_level(Level::Low)
            .with_idle_output(true)
            .with_carrier_modulation(false);
        let channel = channel
            .configure_tx(&config)
            .expect("Failed to configure RMT TX channel")
            .with_pin(pin);

        Self {
            rmt_channel: Some(channel),
            rmt_buffer: [PulseCode::end_marker(); RMT_BUF_SIZE],
        }
    }

    /// Send one frame to the chain, blocking until transmission completes.
    pub fn write(&mut self, pixels: &[ColorRGBW; LED_COUNT]) {
        let mut i = 0;
        for pixel in pixels {
            // SK6812 takes its data MSB-first in G, R, B, W channel order.
            for byte in [pixel.g, pixel.r, pixel.b, pixel.w] {
                for bit in 0..8 {
                    self.rmt_buffer[i] = if (byte & (0x80u8 >> bit)) != 0 { ONE } else { ZERO };
                    i += 1;
                }
            }
        }
        self.rmt_buffer[i] = PulseCode::end_marker();

        // transmit/wait consume and hand back the channel; keep it for next time.
        let channel = self.rmt_channel.take().unwrap();
        let channel = match channel.transmit(&self.rmt_buffer) {
            Ok(tx) => tx.wait().unwrap_or_else(|(_, ch)| ch),
            Err((_, ch)) => ch,
        };
        self.rmt_channel = Some(channel);
    }
}
