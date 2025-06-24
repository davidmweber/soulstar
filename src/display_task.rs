use crate::led_driver::LedDriver;
use crate::tracker::Tracker;
use embassy_futures::select::{Either3::*, select3};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_time::{Duration, Instant, Ticker};
use heapless::String;
use log::info;
use smart_leds::RGB8;

/// A message containing presence information from a detected nearby device
pub struct PresenceMessage {
    /// Received Signal Strength Indicator in dBm, indicating signal strength
    #[allow(unused)]
    pub rssi: i8,
    /// Unique identifier address of the detected device
    pub address: u32,
    /// When did we receive an update for this message
    pub last_seen: Instant,
    /// The name advertised in the beacon
    #[allow(unused)]
    pub name: String<24>,
}

/// Manage the display state by sending it messages of this type. If anyone asks why I like Rust,
/// this is one of the many reasons
pub enum DisplayState {
    /// Suspends animation update
    #[allow(unused)]
    Stop,
    /// Restart animation update
    #[allow(unused)]
    Start,
    /// Set the pixel colour. It is always the 1st pixel. Boring but....
    #[allow(unused)]
    Colour(RGB8),
    /// Switch of all the LEDs
    #[allow(unused)]
    Off,
    /// Sets the LED to torch mode. This disables the animation
    #[allow(unused)]
    Torch(u8),
    /// A message sent from the bluetooth controller containing beacon data for another device
    #[allow(unused)]
    Presence(PresenceMessage),
}

const DISPLAY_QUEUE_SIZE: usize = 10;
/// Channel types for the display task.
pub type DisplayChannel = Channel<CriticalSectionRawMutex, DisplayState, DISPLAY_QUEUE_SIZE>;
pub type DisplayChannelSender =
    Sender<'static, CriticalSectionRawMutex, DisplayState, DISPLAY_QUEUE_SIZE>;
pub type DisplayChannelReceiver =
    Receiver<'static, CriticalSectionRawMutex, DisplayState, DISPLAY_QUEUE_SIZE>;

/// Display driver main task.
/// The display is fully managed from this task. It contains the state and responds to messages
/// sent to it via the channel.
///
#[embassy_executor::task]
pub async fn display_task(channel: &'static DisplayChannelReceiver, led: &'static mut LedDriver) {
    let mut ticker = Ticker::every(Duration::from_millis(100));
    let mut flusher = Ticker::every(Duration::from_secs(60));
    let mut running = true;
    let mut tracker: Tracker<32> = Tracker::new();
    info!("DISPLAY_TASK: Task started. Waiting for messages...");
    loop {
        match select3(ticker.next(), channel.receive(), flusher.next()).await {
            First(_) => {
                // The ticker woke us up
                if running {
                    led.rotate_left();
                    led.update_string();
                }
            }
            Second(message) => {
                // We received a message
                use DisplayState::*;
                match message {
                    Stop => running = false,
                    Start => running = true,
                    Colour(c) => {
                        led.all_off();
                        led.buffer[0] = c;
                        led.update_string();
                    }
                    Off => {
                        led.all_off();
                        led.update_string();
                        running = false;
                    }
                    Torch(c) => {
                        led.torch(c);
                        led.update_string();
                        running = false;
                    }
                    Presence(message) => {
                        tracker.update(message);
                    }
                }
            }
            Third(_) => tracker.flush(),
        };
    }
}
