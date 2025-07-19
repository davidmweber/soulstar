use crate::animations::{Animation, PresenceAnimation, SparkleAnimation, is_interruptable, next_buffer};
use crate::configuration::*;
use crate::led_driver::{LedBuffer, LedDriver};
use crate::presence::PresenceMessage;
use crate::tracker::Tracker;
use defmt::{debug, info};
use embassy_futures::select::{Either3::*, select3};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_time::{Duration, Ticker};
use heapless::spsc::Queue;

/// Manage the display state by sending it messages of this type. If anyone asks why I like Rust,
/// this is one of the many reasons
#[allow(unused)]
pub enum DisplayState {
    /// Suspends animation update
    Stop,
    /// Restart animation update
    Start,
    /// Switch of all the LEDs, stopping animation
    Off,
    /// Start the animation again
    On,
    /// Enable/disable torch function
    Torch(bool),
    /// Set the display brightness
    Brightness(u8),
    /// Update the presence with a newly received BLE advertisement
    PresenceUpdate(PresenceMessage),
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
    led: &'static mut LedDriver,
    default: &'static Animation,
) {
    let mut animation = Ticker::every(Duration::from_millis(ANIMATION_UPDATE));
    let mut flusher = Ticker::every(Duration::from_secs(PRESENCE_REGISTER_FLUSH_INTERVAL));
    let mut running = true;
    let mut tracker: Tracker<MAX_SOULS_TRACKED> = Tracker::new();
    let mut animation_queue: Queue<Animation, MAX_PENDING_ANIMATIONS> = Queue::new();
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
                    // Look at our state and return something that we can display.
                    // Note we must peek into animation_queue because if we are interruptable, we must
                    // leave the next animation in the queue until the current animation terminates.
                    let mut new_buf: Option<LedBuffer> = match (
                        next_buffer(&mut current_animation),
                        animation_queue.peek(),
                        is_interruptable(&current_animation),
                    ) {
                        // A new animation and the current one is interruptable, set up the new one.
                        (_, Some(animation), true) => {
                            debug!("DISPLAY_TASK: Animation {} replaced by updated {}", current_animation, animation);
                            current_animation = animation.clone();
                            animation_queue.dequeue().unwrap(); // Infallible drop because the peek was Some()
                            next_buffer(&mut current_animation)
                        }
                        // Just one animation running, so let it roll
                        (Some(buf), None, _) => {
                            debug!("DISPLAY_TASK: Animation continuing with {}", current_animation);
                            Some(buf)
                        }
                        // A new animation available but we are not interruptable, return the current animation next buffer
                        (Some(buf), Some(animation), false) => {
                            debug!(
                                "DISPLAY_TASK: Uninterruptible animation {} updated with pending animation {}",
                                current_animation, animation
                            );
                            Some(buf)
                        }
                        // Current animation terminates, no new animation so revert to default
                        (None, None, _) => {
                            debug!("DISPLAY_TASK: No animations found. Reverting to the default");
                            current_animation = default.clone();
                            next_buffer(&mut current_animation)
                        }
                        // No new buffer and a pending animation
                        (None, Some(animation), _) => {
                            debug!("DISPLAY_TASK: No current animation with a pending animation {}", animation);
                            current_animation = animation.clone();
                            animation_queue.dequeue().unwrap(); // Infallible drop because the peek was Some()
                            next_buffer(&mut current_animation)
                        }
                    };
                    // The buffer is still wrapped in an option, so grab it. It will never be None
                    if let Some(ref mut b) = new_buf {
                        led.update_from_buffer(b, brightness).await;
                    } // Just let the default animation pick this one up if we don't have a new buffer
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
                            led.torch(brightness).await;
                        } else {
                            running = true;
                        };
                    }
                    PresenceUpdate(message) => {
                        // Only update if there was a change to the presence list. The update()
                        // method returns true if there was an update.
                        if tracker.update(&message).await {
                            info!("DISPLAY_TASK: Presence update message received!");
                            let souls = tracker.get_soul_summary().await;
                            // Send sparkle animation for new user. There can only be one
                            animation_queue
                                .enqueue(Animation::Sparkle(SparkleAnimation::new(
                                    message.colour,
                                    Some(Duration::from_secs(NEW_SOUL_ANIMATION)),
                                )))
                                .unwrap_or(());
                            // Silently drop an animation if the queue is full
                            animation_queue
                                .enqueue(Animation::Presence(PresenceAnimation::new(&souls)))
                                .unwrap_or(());
                        };
                    }
                }
            }
            // Flush stale presence messages timer
            Third(_) => {
                if tracker.flush().await {
                    // Someone disappeared so update the animation
                    info!("DISPLAY_TASK: A soul disappeared");
                    let souls = tracker.get_soul_summary().await;
                    animation_queue
                        .enqueue(Animation::Presence(PresenceAnimation::new(&souls)))
                        .unwrap_or(());
                }
            }
        };
    }
}
