#![no_std]
#![no_main]

mod tm1637;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
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

    loop {
        tm.with_digits([SevenSegmentChar::Digit0; 4]).with_display_on().refresh().await;
        for (i, level) in [BrightnessLevel::Level0, BrightnessLevel::Level1, BrightnessLevel::Level2, BrightnessLevel::Level3, BrightnessLevel::Level4, BrightnessLevel::Level5, BrightnessLevel::Level6, BrightnessLevel::Level7].iter().enumerate() {
            tm.with_brightness_level(*level).refresh().await;
            Timer::after(Duration::from_millis(250)).await;
        }

        for i in 0..10{
            tm.with_colon_on(i%2 == 0).refresh().await;
            Timer::after(Duration::from_millis(250)).await;
        }

        for i in 0..10000 {
            let mut digits = [SevenSegmentChar::Blank; 4];
            for (index, digit) in digits.iter_mut().rev().enumerate() {
                *digit = (((i / (10_i32.pow(index as u32))) % 10) as u8).into()
            }
            for digit in digits.iter_mut() {
                if *digit == SevenSegmentChar::Digit0 {
                    *digit = SevenSegmentChar::Blank;
                } else {
                    break;
                }
            }
            tm.with_digits(digits).refresh().await;
            Timer::after(Duration::from_millis(10)).await;
        }

        tm.with_display_off().refresh().await;
        Timer::after(Duration::from_secs(1)).await;
        info!("Tick");
    }
}

