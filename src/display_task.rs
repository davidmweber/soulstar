use embassy_futures::select::{select, Either};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use smart_leds::RGB8;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Ticker};
use log::info;
use crate::led_driver::LedDriver;

/// Manage the display state by sending it messages of this type. If anyone asks why I like Rust,
/// this is one of the many reasons
pub enum DisplayState {
    /// Suspends animation update
    Stop, 
    /// Restart animation update
    Start,
    /// Set the pixel colour. It is always the 1st pixel. Boring but....
    Colour(RGB8),
    /// Switch of all the LEDs
    Off,
    /// Sets the led to torch mode. This disables the animation
    Torch(u8)
}

/// The channel type detail
pub type DisplayControlChannel = Channel<CriticalSectionRawMutex, DisplayState, 3>;


/// Display driver main task.
/// The display is fully managed from this task. It contains the state and responds to messages
/// sent to it via the channel.
#[embassy_executor::task]
pub async fn display_task(channel: &'static DisplayControlChannel, led: &'static mut LedDriver) {
    let mut ticker = Ticker::every(Duration::from_millis(100));
    let mut running = false;
    info!("DISPLAY_TASK: Task started. Waiting for messages...");
    led.update_string();
    loop {
        match select(ticker.next(), channel.receive()).await {
            Either::First(_) => { // The ticker woke us up
                if running {
                    led.rotate_left();
                    led.update_string();
                }
            }
            Either::Second(message) => { // We received a message
                use DisplayState::*;
                match message {
                    Stop => running = false,
                    Start => running = true,
                    Colour(c) => {
                        led.all_off();
                        led.buffer[0] = c;
                        led.update_string();
                    },
                    Off => {
                        led.all_off();
                        led.update_string();
                        running = false;
                    },
                    Torch(c) => {
                        led.torch(c);
                        led.update_string();
                        running = false;
                    }
                    
                    
                }
            }
        };
    }
}