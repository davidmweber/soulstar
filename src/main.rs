#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

extern crate alloc;
mod led_driver;
mod display_task;

use crate::led_driver::LedDriver;
use bt_hci::controller::ExternalController;
use embassy_executor::Spawner;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::timer::timg::TimerGroup;
use esp_wifi::ble::controller::BleConnector;
use log::info;
#[cfg(feature = "log-rtt")]
use rtt_target::rtt_init_log;
use smart_leds::RGB8;
use static_cell::StaticCell;
use crate::display_task::{display_task, DisplayControlChannel};
use crate::display_task::DisplayState::*;

/// Communicate with the display task using this channel and the DisplayState enum
static DISPLAY_CHANNEL: StaticCell<DisplayControlChannel> = StaticCell::new();
static LED_DRIVER: StaticCell<LedDriver> = StaticCell::new();

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    #[cfg(feature = "log-rtt")]
    {
        rtt_init_log!();
        info!("MAIN: Using RTT logging");
    }
    #[cfg(feature = "log-uart")]
    {
        use log::LevelFilter::Info;
        esp_println::logger::init_logger(Info);
        info!("MAIN: Logger initialized: UART (esp-println)");
    }

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timer0 = SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(timer0.alarm0);

    info!("MAIN: Embassy initialized!");

    let rng = esp_hal::rng::Rng::new(peripherals.RNG);
    let timer1 = TimerGroup::new(peripherals.TIMG0);
    let wifi_init = esp_wifi::init(timer1.timer0, rng, peripherals.RADIO_CLK)
        .expect("MAIN: Failed to initialize WIFI/BLE controller");
    
    // find more examples https://github.com/embassy-rs/trouble/tree/main/examples/esp32
    let transport = BleConnector::new(&wifi_init, peripherals.BT);
    let _ble_controller = ExternalController::<_, 20>::new(transport);

    info!("MAIN: Setting up LED driver controller");
    let display_channel = DISPLAY_CHANNEL.init(Channel::new());
    let led_driver: &'static mut LedDriver = LED_DRIVER.init(LedDriver::new(peripherals.RMT, peripherals.GPIO6));
    spawner.spawn(display_task(display_channel, led_driver)).expect("Failed to spawn display task");
    
    // Simple example that exercises the display task
    loop {
        display_channel.send(Off).await;
        display_channel.send(Colour(RGB8::new(0,10,10))).await;
        display_channel.send(Start).await;

        Timer::after(Duration::from_secs(10)).await;
        display_channel.send(Stop).await;

        Timer::after(Duration::from_secs(1)).await;
        display_channel.send(Start).await;
        
        Timer::after(Duration::from_secs(1)).await;
        display_channel.send(Torch(10)).await;

        Timer::after(Duration::from_secs(1)).await;
        display_channel.send(Torch(20)).await;

        Timer::after(Duration::from_secs(1)).await;
    }

    // for inspiration, have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0-beta.1/examples/src/bin
}
