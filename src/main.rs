#![no_std]
#![no_main]

mod sk6812;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::{
    interrupt::software::SoftwareInterruptControl, rmt::Rmt, time::Rate, timer::timg::TimerGroup,
};
use log::info;

// Provide #[panic_handler]
use esp_backtrace as _;

use sk6812::{ColorRGBW, SK6812ChainDriver};

esp_bootloader_esp_idf::esp_app_desc!();

const LED_COUNT: usize = 4;

#[embassy_executor::task]
async fn run() {
    loop {
        info!("From task!");
        Timer::after(Duration::from_millis(1_000)).await;
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    info!("Init!");

    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

    let rmt =
        Rmt::new(peripherals.RMT, Rate::from_mhz(80)).expect("Failed to initialize RMT peripheral");
    let mut leds = SK6812ChainDriver::<LED_COUNT, { sk6812::frame_len(LED_COUNT) }>::new(
        rmt.channel0,
        peripherals.GPIO3,
    );

    spawner.spawn(run().unwrap());

    loop {
        info!("Blink from main!");
        Timer::after(Duration::from_millis(1_000)).await;
        leds.write(&[
            ColorRGBW::new(0, 60, 0, 0),
            ColorRGBW::new(60, 0, 0, 0),
            ColorRGBW::new(0, 0, 60, 0),
            ColorRGBW::new(60, 60, 0, 0),
        ]);
        Timer::after(Duration::from_millis(1_000)).await;
        leds.write(&[ColorRGBW::OFF; LED_COUNT]);
    }
}
