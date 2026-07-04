#![no_std]
#![no_main]

mod leds;
mod buttons;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::{
    interrupt::software::SoftwareInterruptControl, rmt::Rmt, time::Rate, timer::timg::TimerGroup,
};
use log::info;

// Provide #[panic_handler]
use esp_backtrace as _;

use buttons::Buttons;
use leds::{ColorRGBW, SK6812ChainDriver};

esp_bootloader_esp_idf::esp_app_desc!();

const LED_COUNT: usize = 4;

/// Color lit for each button when pressed, indexed by LED position to match
/// the button table in `buttons` (green, red, blue, yellow).
const LED_BRIGHTNESS: u8 = 200;
const COLORS: [ColorRGBW; LED_COUNT] = [
    ColorRGBW::new(0, LED_BRIGHTNESS, 0, 0),
    ColorRGBW::new(LED_BRIGHTNESS, 0, 0, 0),
    ColorRGBW::new(0, 0, LED_BRIGHTNESS, 0),
    ColorRGBW::new(LED_BRIGHTNESS, LED_BRIGHTNESS, 0, 0),
];

#[embassy_executor::task]
async fn run() {
    loop {
        info!("From run task!");
        Timer::after(Duration::from_millis(1_000)).await;
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    info!("ESP HAL init done");

    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

    info!("ESP RTOS scheduler started");

    spawner.spawn(run().unwrap());

    let rmt =
        Rmt::new(peripherals.RMT, Rate::from_mhz(80)).expect("Failed to initialize RMT peripheral");
    let mut leds = SK6812ChainDriver::<LED_COUNT, { leds::frame_len(LED_COUNT) }>::new(
        rmt.channel0,
        peripherals.GPIO3,
    );

    let buttons = Buttons::new(
        peripherals.GPIO11,
        peripherals.GPIO10,
        peripherals.GPIO6,
        peripherals.GPIO7,
    );

    loop {
        let pressed = buttons.pressed();
        let mut frame = [ColorRGBW::OFF; LED_COUNT];
        for (led, &is_pressed) in pressed.iter().enumerate() {
            if is_pressed {
                frame[led] = COLORS[led];
            }
        }
        leds.write(&frame);
        Timer::after(Duration::from_millis(20)).await;
    }
}
