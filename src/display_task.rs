use crate::animations::Animation;
use crate::configuration::{ANIMATION_UPDATE, MAX_SOULS_TRACKED};
use crate::led_driver::LedDriver0;
use crate::presence::PresenceMessage;
use crate::tracker::Tracker;
use defmt::info;
use embassy_futures::select::{Either3::*, select3};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_time::{Duration, Ticker};
use heapless::spsc::Queue;

/// Manage the display state by sending it messages of this type. If anyone asks why I like Rust,
/// this is one of the many reasons
pub enum DisplayState {
    /// Suspends animation update
    Stop,
    /// Restart animation update
    Start,
    /// Switch of all the LEDs, stopping animation
    Off,
    /// Start the animation again
    On,
    /// Sets the LED to torch mode. This disables the animation
    Torch(bool),
    /// Set the overall brightness of the animation
    Brightness(u8),
    /// A message sent from the bluetooth controller containing beacon data for another device
    Presence(PresenceMessage),
}

const DISPLAY_QUEUE_SIZE: usize = 10;
/// Channel types for the display task.
pub type DisplayChannel = Channel<CriticalSectionRawMutex, DisplayState, DISPLAY_QUEUE_SIZE>;
pub type DisplayChannelSender = Sender<'static, CriticalSectionRawMutex, DisplayState, DISPLAY_QUEUE_SIZE>;
pub type DisplayChannelReceiver = Receiver<'static, CriticalSectionRawMutex, DisplayState, DISPLAY_QUEUE_SIZE>;

/// Display driver main task.
/// The display is fully managed from this task. It contains the state and responds to messages
/// sent to it via the channel.
///
/// # Parameters
/// * `channel` - Channel receiver for display state messages
/// * `led` - LED driver instance for controlling the LED strip
/// * `default` - Default animation type to use when no other animation is queued. T
///
#[embassy_executor::task]
pub async fn display_task(
    channel: &'static DisplayChannelReceiver,
    led: &'static mut LedDriver0,
    default: &'static Animation,
) {
    let mut animation = Ticker::every(Duration::from_millis(ANIMATION_UPDATE));
    let mut flusher = Ticker::every(Duration::from_secs(10));
    let mut running = true;
    let mut tracker: Tracker<MAX_SOULS_TRACKED> = Tracker::new();
    let mut animation_queue: Queue<Animation, 10> = Queue::new();
    let mut current_animation = default.clone();
    let mut brightness: u8 = 128;

    info!("DISPLAY_TASK: Task started. Waiting for messages...");
    loop {
        // Wait for one of our futures to become ready
        match select3(animation.next(), channel.receive(), flusher.next()).await {
            // Animation update timer
            First(_) => {
                // The ticker woke us up
                if running {
                    // If our queue is empty, just carry on. If there is something
                    // in the queue and the current animation is interruptable. drop it
                    // and start the next animation or just carry on until it times out.
                    let mut buffer = match &mut current_animation {
                        Animation::Sparkle(s) => s.next(),
                    };
                    if let Some(ref mut b) = buffer {
                        led.update_from_buffer(b, brightness);
                    } else {
                        current_animation = match animation_queue.dequeue() {
                            Some(a) => a,
                            None => default.clone(),
                        };
                    }
                }
            }
            // Control message from our channel
            Second(message) => {
                // We received a message
                use DisplayState::*;
                match message {
                    Stop => running = false,
                    Start => running = true,
                    Off => {
                        led.all_off();
                        running = false;
                    }
                    On => {
                        running = true;
                    }
                    Brightness(b) => {
                        brightness = b;
                    }
                    Torch(on) => {
                        if on {
                            running = false;
                            led.torch(brightness);
                        } else {
                            running = true;
                        };
                    }
                    Presence(message) => {
                        // Only update if there was a change to the presence list. The update()
                        // method returns true if there was an update.
                        if tracker.update(message).await {
                            info!("Presence update message received!");
                            // send sparkle for new animation
                            // Update presence message
                        }
                    }
                }
            }
            // FLush stale presence messages timer
            Third(_) => {
                if tracker.flush().await {
                    // Someone disappeared so update the animation
                    info!("A soul disapeared");
                }
            }
        };
    }
}
