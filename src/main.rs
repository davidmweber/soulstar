#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

mod colour;
mod configuration;
mod display_task;
mod led_driver;
mod presence;
mod soul_config;
mod tracker;

use crate::display_task::DisplayState::*;
use crate::display_task::{DisplayChannel, DisplayChannelReceiver, DisplayChannelSender, display_task};
use crate::led_driver::LedDriver;
use crate::presence::{BleControllerType, start_ble};
use embassy_executor::Spawner;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::timer::timg::TimerGroup;
use esp_wifi::ble::controller::BleConnector;
use smart_leds::RGB8;
use static_cell::StaticCell;

use defmt::*;
use defmt_rtt as _;
// Global logger + panicking-behavior + memory layout
use esp_backtrace as _;
use esp_println as _;

/// Tasks require `static types to guarantee their life-time as the task can outlive
/// the main process. Basically anything that is a parameter for an Embassy task must
/// be managed bu a StaticCell
/// Communicate with the display task using this channel and the DisplayState enum
static DISPLAY_SENDER: StaticCell<DisplayChannelSender> = StaticCell::new();
static DISPLAY_RECEIVER: StaticCell<DisplayChannelReceiver> = StaticCell::new();
static DISPLAY_CHANNEL: StaticCell<DisplayChannel> = StaticCell::new();

/// Our LED driver that underlies the display task
static LED_DRIVER: StaticCell<LedDriver> = StaticCell::new();

/// WiFo configuration that is used by the BLE stack
static WIFI_INIT: StaticCell<esp_wifi::EspWifiController> = StaticCell::new();

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressiReceiver<CriticalSectionRawMutex, DisplayState, 3>e/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    info!("MAIN: Starting up Soul Star for {}", soul_config::ADVERTISED_NAME);

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timer0 = SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(timer0.alarm0);
    let display_channel = DISPLAY_CHANNEL.init(Channel::new());
    let sender = display_channel.sender();
    let ble_sender = DISPLAY_SENDER.init(sender);
    let receiver = DISPLAY_RECEIVER.init(display_channel.receiver());

    info!("MAIN: Setting up the BLE controller");

    let rng = esp_hal::rng::Rng::new(peripherals.RNG);
    let timer1 = TimerGroup::new(peripherals.TIMG0);
    let wifi_init = WIFI_INIT.init(esp_wifi::init(timer1.timer0, rng, peripherals.RADIO_CLK).unwrap());

    let connector = BleConnector::new(wifi_init, peripherals.BT);
    let ble_controller = BleControllerType::new(connector);
    spawner.spawn(start_ble(ble_controller, ble_sender)).unwrap();

    info!("MAIN: Setting up LED driver controller");
    let led_driver: &'static mut LedDriver = LED_DRIVER.init(LedDriver::new(peripherals.RMT, peripherals.GPIO6));
    // Start the display manager task
    spawner
        .spawn(display_task(receiver, led_driver))
        .expect("Failed to spawn display task");

    // Simple example that exercises the display task
    sender.send(Colour(RGB8::new(0, 10, 0))).await;
    sender.send(Stop).await;
    info!("MAIN: Starting main loop");

    loop {
        Timer::after(Duration::from_secs(5)).await;
        // sender.send(Colour(RGB8::new(0, 0, 10))).await;
        // sender.send(FlipAnimation).await;
        // Timer::after(Duration::from_secs(5)).await;
        // sender.send(Colour(RGB8::new(10, 0, 0))).await;
        // sender.send(FlipAnimation).await;
        //trace!("MAIN: Mail loop ticker ticked");
    }
}
