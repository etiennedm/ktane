#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::{interrupt::software::SoftwareInterruptControl, timer::timg::TimerGroup, rmt::Rmt};
use esp_println::println;
use esp_hal_smartled::{buffer_size_async, buffer_size_rgbw, smart_led_buffer, SmartLedsAdapter, SmartLedsAdapterAsync};
use smart_leds::{SmartLedsWrite, RGBW, RGBA};

// Provide #[panic_handler]
use esp_backtrace as _;
use esp_hal::time::Rate;

esp_bootloader_esp_idf::esp_app_desc!();

#[embassy_executor::task]
async fn run() {
    loop {
        println!("Hello world from embassy!");
        Timer::after(Duration::from_millis(1_000)).await;
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    println!("Init!");

    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

    const LED_COUNT: usize = 4;
    let rmt: Rmt<'_, esp_hal::Blocking> = {
        let frequency = Rate::from_mhz(80);
        Rmt::new(peripherals.RMT, frequency)
    }.expect("Failed to initialize RMT peripheral");

    let rmt_channel = rmt.channel0;
    let mut rmt_buffer = [esp_hal::rmt::PulseCode::default(); buffer_size_rgbw(LED_COUNT)];
    let mut led: SmartLedsAdapter<'_, { buffer_size_rgbw(LED_COUNT) }, RGBA<u8>> = SmartLedsAdapter::new_with_color(rmt_channel, peripherals.GPIO3, &mut rmt_buffer);

    spawner.spawn(run()).ok();

    loop {
        println!("Bing!");
        Timer::after(Duration::from_millis(1_000)).await;
        led.write(
            [
                RGBA::new(60, 0, 0, 0),
                RGBA::new(0, 60, 0, 0),
                RGBA::new(0, 0, 60, 0),
                RGBA::new(60, 60, 0, 0),
            ]
        ).unwrap();
        Timer::after(Duration::from_millis(1_000)).await;
        led.write(
            [
                RGBA::new(0, 0, 0, 0),
                RGBA::new(0, 0, 0, 0),
                RGBA::new(0, 0, 0, 0),
                RGBA::new(0, 0, 0, 0),
            ]
        ).unwrap();
    }
}