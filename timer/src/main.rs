#![no_std]
#![no_main]

mod tm1637;

use embassy_executor::Spawner;
use embassy_time::{Duration, Instant, Timer};
use esp_hal::{
    interrupt::software::SoftwareInterruptControl, timer::timg::TimerGroup, time::Rate,
};
use esp_hal::i2c::master::{Config, I2c};
use log::info;

use tm1637::{BrightnessLevel, SevenSegmentChar, TM1637};

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

    let i2c_config = Config::default().with_frequency(Rate::from_khz(100));
    let i2c0 = I2c::new(peripherals.I2C0, i2c_config)
        .unwrap()
        .into_async()
        .with_sda(peripherals.GPIO5)
        .with_scl(peripherals.GPIO4);

    let mut tm = TM1637::new(i2c0);

    info!("ESP RTOS scheduler started");

    let mut time_digits = [SevenSegmentChar::Blank; 4];
    let mut previous_seconds = 0u64;

    loop {
        let elapsed_seconds = Instant::now().as_secs();

        if previous_seconds != elapsed_seconds {
            let minutes = ((elapsed_seconds / 60) % 60) as u8;
            let seconds = (elapsed_seconds % 60) as u8;
            info!("Tick, it is now {minutes:02}:{seconds:02}");

            time_digits[3] = (seconds % 10).into();
            time_digits[2] = (seconds / 10).into();
            time_digits[1] = (minutes % 10).into();
            time_digits[0] = (minutes / 10).into();

            previous_seconds = elapsed_seconds;
        }

        tm.with_digits(time_digits);
        tm.with_colon_on(true);
        tm.with_brightness_level(BrightnessLevel::Level5).with_display_on().refresh().await;
        Timer::after(Duration::from_millis(100)).await;
    }
}

