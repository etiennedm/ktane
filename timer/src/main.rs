#![no_std]
#![no_main]

mod tm1637;

use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Instant, Timer};
use embassy_futures::select::select;
use embedded_can::Id::{Extended, Standard};
use esp_hal::{Async, interrupt::software::SoftwareInterruptControl, timer::timg::TimerGroup, time::Rate, twai, efuse};
use esp_hal::i2c::master::{Config, I2c};
use esp_hal::twai::{TwaiMode, TwaiRx, TwaiTx, StandardId, EspTwaiFrame};
use embedded_can::Frame;
use log::{error, info};

use zencan_node::object_dict::ObjectAccess;
use zencan::OBJECT2000;
use zencan_node::{common::NodeId, Node, Callbacks};

use tm1637::{BrightnessLevel, SevenSegmentChar, TM1637};

mod zencan {
    zencan_node::include_modules!(ZENCAN_CONFIG);
}

// Provide #[panic_handler]
use esp_backtrace as _;

esp_bootloader_esp_idf::esp_app_desc!();

static CANOPEN_PROCESS_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();
static CANOPEN_TX_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
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

    let twai_config = twai::TwaiConfiguration::new(
        peripherals.TWAI0,
        peripherals.GPIO0,
        peripherals.GPIO2,
        twai::BaudRate::B125K,
        TwaiMode::Normal,
    );
    let (twai_rx, twai_tx) = twai_config.into_async().start().split();

    info!("ESP RTOS scheduler started");

    let mac_address = efuse::base_mac_address();
    info!("MAC address: {:?}", mac_address.as_bytes());

    let last_mac_bytes: [u8; 4] = mac_address.as_bytes()[2..].try_into().unwrap();
    let serial = u32::from_be_bytes(last_mac_bytes);

    zencan::OBJECT1018.set_serial(serial);
    zencan::NODE_MBOX.set_process_notify_callback(&notify_canopen_process_task);
    zencan::NODE_MBOX.set_transmit_notify_callback(&notify_canopen_tx_task);

    spawner.spawn(twai_rx_task(twai_rx).unwrap());
    spawner.spawn(twai_tx_task(twai_tx).unwrap());
    spawner.spawn(canopen_process_task().unwrap());

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

            OBJECT2000.set_value(elapsed_seconds as u32);
            let _ = OBJECT2000.set_event_flag(0);
        }

        tm.with_digits(time_digits);
        tm.with_colon_on(true);
        tm.with_brightness_level(BrightnessLevel::Level5).with_display_on().refresh().await;
        Timer::after(Duration::from_millis(100)).await;
    }
}


fn notify_canopen_process_task() {
    CANOPEN_PROCESS_SIGNAL.signal(());
}

fn notify_canopen_tx_task() {
    CANOPEN_TX_SIGNAL.signal(());
}

#[embassy_executor::task]
async fn canopen_process_task() {
    let mac_address = efuse::base_mac_address();
    let node_id = match *mac_address.as_bytes().last().unwrap() {
        0 => 1,
        id => id,
    };

    let mut node = Node::new(
        NodeId::new(node_id).unwrap(),
        Callbacks::default(),
        &zencan::NODE_MBOX,
        &zencan::NODE_STATE,
        &zencan::OD_TABLE,
    );

    loop {
        select(CANOPEN_PROCESS_SIGNAL.wait(), Timer::after_millis(10)).await;
        let now_us = Instant::now().as_micros();
        // info!("Start to process CANopen @{}.{}ms", now_us/1000, now_us % 1000);

        node.process(now_us);
    }
}

#[embassy_executor::task]
async fn twai_tx_task(mut twai_tx: TwaiTx<'static, Async>) {
    loop {
        while let Some(msg) = zencan::NODE_MBOX.next_transmit_message() {
            let frame = match
                EspTwaiFrame::new(StandardId::new(msg.id.raw() as u16).unwrap(), msg.data()) {
                Some(frame) => frame,
                None => {
                    error!("Failed to create TX frame for message: {:?}", msg);
                    continue;
                },
            };

            // info!("Sending frame: {:?}", frame);
            if let Err(e) = twai_tx.transmit_async(&frame).await {
                error!("Error sending CAN message: {e:?}");
            }
        }

        // Wait for wakeup signal when new CAN messages become ready for sending
        CANOPEN_TX_SIGNAL.wait().await;
    }
}

#[embassy_executor::task]
async fn twai_rx_task(mut twai_rx: TwaiRx<'static, Async>) {
    loop {
        let frame = match twai_rx.receive_async().await {
            Ok(rx_frame) => rx_frame,
            Err(e) => {
                error!("Error receiving frame: {e:?}");
                continue;
            }
        };
        // info!("Received frame: {:?}", frame);

        let id = match frame.id() {
            Standard(id) => zencan_node::common::messages::CanId::std(id.as_raw()),
            Extended(id) => zencan_node::common::messages::CanId::extended(id.as_raw()),
        };

        let msg = zencan_node::common::messages::CanMessage::new(id, frame.data());
        zencan::NODE_MBOX.store_message(msg).ok();
    }
}
