#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::{
    interrupt::software::SoftwareInterruptControl, timer::timg::TimerGroup,
};
use log::info;

// Provide #[panic_handler]
use esp_backtrace as _;

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(_spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    let peripherals = esp_hal::init(esp_hal::Config::default());

    info!("ESP HAL init done");

    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

    info!("ESP RTOS scheduler started");

    loop {
        info!("Running...");
        Timer::after(Duration::from_secs(1)).await;
    }
}

