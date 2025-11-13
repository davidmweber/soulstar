//! Track presence messages
//! Provides basic tools to update new presences and delete expired presences.
//! This module manages a list of active presences, their associated colors, and handles
//! their lifecycle including addition, updates, and expiration.

use crate::configuration::{MAX_SOULS_TRACKED, TRACKER_FLUSH_AGE};
use crate::presence::PresenceMessage;
use defmt::{Debug2Format, error, info};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant};
use heapless::Vec;
use heapless::index_map::FnvIndexMap;
use smart_leds::RGB8;
use trouble_host::prelude::BdAddr;

pub type PresenceMap<const S: usize> = FnvIndexMap<u32, PresenceMessage, S>;
type PresenceMutex<const S: usize> = Mutex<NoopRawMutex, PresenceMap<S>>;

/// We want a u32 that sort of uniquely identifies the sender's "MAC" address. As we set this
/// to some random value, we will have unique key for the hash that we store
fn addr_to_key(addr: &BdAddr) -> u32 {
    let r = addr.raw();
    r[5] as u32 | (r[4] as u32) << 8 | ((r[3] ^ r[1]) as u32) << 16 | ((r[2] ^ r[0]) as u32) << 24
}

#[derive(Clone, Debug)]
#[allow(unused)]
pub struct SoulSummary {
    pub colour: RGB8,
    pub tx_loss: i32,
}

pub type VisibleSouls = Vec<SoulSummary, { MAX_SOULS_TRACKED }>;

/// A tracker that manages a fixed-size collection of presence messages.
/// Each presence message represents a connected device (soul) with its associated
/// properties like name, colour, and last seen timestamp.
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
    pub async fn update(&mut self, presence: &PresenceMessage) -> bool {
        let addr = presence.address;
        let name = presence.name.clone();
        let mut guard = self.souls.lock().await;
        match guard.insert(addr_to_key(&addr), presence.clone()) {
            Ok(Some(_)) => false, // Already present, but we may have an updated RSSI, so at some point, we want to react to the RSSI change
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

    /// Retrieve the information that would be used by an animation. So just colour and the
    /// transmitter power.
    pub async fn get_soul_summary(&self) -> VisibleSouls {
        let guard = self.souls.lock().await;
        guard
            .iter()
            .map(|(_, p)| SoulSummary {
                colour: p.colour,
                tx_loss: p.tx_power as i32 - p.rssi as i32,
            })
            .collect()
    }

    /// Flush all presence entries that are older than the time specified in the argument
    pub async fn flush(&mut self) -> bool {
        // If our first flush happens in less time than our uptime, this crashes
        if let Some(horizon) = Instant::now().checked_sub(Duration::from_secs(TRACKER_FLUSH_AGE)) {
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
}
