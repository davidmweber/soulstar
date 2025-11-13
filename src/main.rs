#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
extern crate alloc;

mod animations;
mod button;
mod colour;
mod configuration;
mod display_task;
mod led_driver;
mod presence;
mod soul_config;
mod throbber;
mod tracker;

use crate::display_task::{DisplayChannel, DisplayChannelReceiver, DisplayChannelSender, display_task};
use crate::led_driver::LedDriver;
use crate::presence::start_ble;
use bt_hci::controller::ExternalController;
use core::panic::PanicInfo;
use embassy_executor::Spawner;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::timer::systimer::SystemTimer;
use esp_radio::ble::controller::BleConnector;
use smart_leds::RGB8;
use static_cell::StaticCell;

use crate::animations::Animation::Sparkle;
use crate::animations::{Animation, SparkleAnimation};
use crate::button::wait_for_press;
use crate::colour::clip;
use crate::display_task::DisplayState::{Brightness, Torch};
use defmt::info;

use embassy_futures::select::Either3::{First, Second, Third};
use embassy_futures::select::select3;
use esp_hal::gpio::{Input, InputConfig, Pull};
use esp_hal::rmt::Rmt;
use esp_hal::rng::Rng;
use esp_hal::time::Rate;
use rand_core::RngCore;
use trouble_host::Address;

use defmt_rtt as _;

/// Tasks require `static types to guarantee their life-time as the task can outlive
/// the main process. Basically anything that is a parameter for an Embassy task must
/// be managed bu a StaticCell
/// Communicate with the display task using this channel and the DisplayState enum
static DISPLAY_SENDER: StaticCell<DisplayChannelSender> = StaticCell::new();
static DISPLAY_RECEIVER: StaticCell<DisplayChannelReceiver> = StaticCell::new();
static DISPLAY_CHANNEL: StaticCell<DisplayChannel> = StaticCell::new();

/// Our LED driver that underlies the display task
static LED_DRIVER: StaticCell<LedDriver> = StaticCell::new();

/// Wi-Fi configuration that is used by the BLE stack
static RADIO_INIT: StaticCell<esp_radio::Controller> = StaticCell::new();

/// Set a random MAC address for this beacon.
static ADDRESS: StaticCell<Address> = StaticCell::new();

/// Our default animation
static DEFAULT_ANIMATION: StaticCell<Animation> = StaticCell::new();

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    defmt::error!("PANIC: {}", defmt::Debug2Format(info));
    loop {}
}

// This creates a default app-descriptor required by the esp-idf bootloader.
esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) {
    // Set up Embassy and start the executor
    info!("MAIN: Starting up Soul Star for {}", soul_config::ADVERTISED_NAME);
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    esp_alloc::heap_allocator!(size: 64 * 1024);
    let timer0 = SystemTimer::new(peripherals.SYSTIMER);
    let sw_interrupt = esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timer0.alarm0, sw_interrupt.software_interrupt0);

    // Set up the communication channels that we use for IPC
    let display_channel = DISPLAY_CHANNEL.init(Channel::new());
    let sender = display_channel.sender();
    let ble_sender = DISPLAY_SENDER.init(sender);
    let receiver = DISPLAY_RECEIVER.init(display_channel.receiver());

    // Configure and start the bluetooth radio stack
    info!("MAIN: Setting up the BLE controller");
    let mut rng = Rng::new();
    let radio_init = RADIO_INIT.init(esp_radio::init().expect("Could not initialize wifi init"));
    // Add delay to ensure the wireless controller is fully initialised before we set up the BLE
    Timer::after(Duration::from_millis(200)).await;

    let transport = BleConnector::new(radio_init, peripherals.BT, Default::default()).unwrap();
    let ble_controller = ExternalController::<_, 20>::new(transport);

    // Random address is recommended for privacy. So each time the device comes to life,
    // it will have a different MAC.
    let mut addr: [u8; 6] = [0, 0, 0, 0, 0, 0];
    rng.fill_bytes(&mut addr);
    let address = ADDRESS.init(Address::random(addr));
    spawner
        .spawn(start_ble(ble_controller, ble_sender, address))
        .expect("Could not start the ble presence task");

    // Kick the RMT peripheral for driving the LED string
    info!("MAIN: Setting up LED driver controller");
    let freq = Rate::from_mhz(80);
    let rmt = Rmt::new(peripherals.RMT, freq).unwrap().into_async();
    let led_driver_0: &'static mut LedDriver = LED_DRIVER.init(LedDriver::new(rmt, peripherals.GPIO6));
    // The initial animation is "Sparkle" with our own colour
    let animation = DEFAULT_ANIMATION.init(Sparkle(SparkleAnimation::new(RGB8::from(soul_config::COLOUR), None)));
    // Start the display manager task
    spawner
        .spawn(display_task(receiver, led_driver_0, animation))
        .expect("Failed to spawn display task");

    // Set up buttons for the functions we need
    let config = InputConfig::default().with_pull(Pull::Up);
    let mut torch_toggle = Input::new(peripherals.GPIO2, config);
    let mut inc_brightness = Input::new(peripherals.GPIO3, config);
    let mut dec_brightness = Input::new(peripherals.GPIO15, config);

    info!("MAIN: Starting main loop");
    sender.send(Brightness(32)).await;
    let mut torch = false;
    let mut brightness = 32u8;
    loop {
        match select3(
            wait_for_press(&mut torch_toggle),
            wait_for_press(&mut inc_brightness),
            wait_for_press(&mut dec_brightness),
        )
        .await
        {
            First(_) => {
                info!("MAIN: Toggling torch mode {}", torch);
                torch ^= true;
                sender.send(Torch(torch)).await;
            }
            Second(_) => {
                info!("MAIN: Increase brightness {}", brightness);
                brightness = clip(brightness as i16 + 16);
                sender.send(Brightness(brightness)).await;
            }
            Third(_) => {
                info!("MAIN: Decrease brightness {}", brightness);
                brightness = clip(brightness as i16 - 16);
                sender.send(Brightness(brightness)).await;
            }
        };
        info!("MAIN: Button pressed");
    }
}
