//! Track presence messages
//! Provides basic tools to update new presences and delete expired presences.
//! This module manages a list of active presences, their associated colors, and handles
//! their lifecycle including addition, updates, and expiration.

use crate::display_task::PresenceMessage;
use defmt::{Debug2Format, error, info};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant};
use heapless::FnvIndexMap;
use smart_leds::RGB8;

pub type PresenceMap<const S: usize> = FnvIndexMap<u32, PresenceMessage, S>;
type PresenceMutex<const S: usize> = Mutex<NoopRawMutex, PresenceMap<S>>;

/// A tracker that manages a fixed-size collection of presence messages.
/// Each presence message represents a connected device (soul) with its associated
/// properties like name, color, and last seen timestamp.
///
/// The generic parameter S determines the maximum number of presences that can be tracked.
pub struct Tracker<const S: usize> {
    pub souls: PresenceMutex<S>,
}

impl<const S: usize> Tracker<S> {
    pub(crate) fn new() -> Self {
        Self {
            souls: Mutex::new(FnvIndexMap::new()),
        }
    }

    /// Updates the tracker with the lastest presence messages
    /// It returns true if the tracker list was updated
    pub async fn update(&mut self, presence: PresenceMessage) -> bool {
        let addr = presence.address;
        let name = presence.name.clone();
        let mut guard = self.souls.lock().await;
        match guard.insert(addr, presence) {
            Ok(Some(_)) => false, // Already present,
            Ok(None) => {
                info!("TRACKER: Adding {} with name {}", Debug2Format(&addr), Debug2Format(&name));
                true
            }
            Err(_) => {
                error!("TRACKER: Error inserting/updating the tracker");
                false
            }
        }
    }

    /// Flush all presence entries that are older than the time specified in the argument
    pub async fn flush(&mut self) -> bool {
        info!("TRACKER: Flushing");
        // If our first flush happens in less time than our uptime, this crashes
        if let Some(horizon) = Instant::now().checked_sub(Duration::from_secs(30)) {
            let mut guard = self.souls.lock().await;
            let len = guard.len();
            guard.retain(|_, v| {
                if v.last_seen > horizon {
                    true
                } else {
                    info!("TRACKER: Removing {} with last presence at {:?}", Debug2Format(&v.name), v.last_seen);
                    false
                }
            });
            return len > guard.len();
        };
        false
    }

    /// Fills an LED buffer with colors from tracked presences.
    /// Each presence's color is copied to the corresponding position in the buffer.
    /// The buffer should be large enough to hold all presence colors.
    ///
    /// # Parameters
    /// * `buffer` - The LED buffer to fill with presence colors
    pub async fn fill_led_buffer(&mut self, buffer: &mut [RGB8]) {
        let guard = self.souls.lock().await;
        for (idx, (_, v)) in guard.iter().enumerate() {
            buffer[idx] = v.color;
        }
    }
}
