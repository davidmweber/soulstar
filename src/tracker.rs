//! Track presence messages
//! Provides basic tools to update new presences and delete expired presences

use crate::display_task::PresenceMessage;
use defmt::{error, info};
use embassy_time::{Duration, Instant};
use heapless::FnvIndexMap;

pub struct Tracker<const S: usize> {
    souls: FnvIndexMap<u32, PresenceMessage, S>,
}

impl<const S: usize> Tracker<S> {
    pub(crate) fn new() -> Self {
        Self {
            souls: FnvIndexMap::new(),
        }
    }

    /// Updates the tracker with the lastest presence messages
    pub fn update(&mut self, presence: PresenceMessage) {
        let addr = presence.address;
        let name = presence.name.clone();
        match self.souls.insert(addr, presence) {
            Ok(Some(_)) => (), // Already present,
            Ok(None) => info!("TRACKER: Adding {}, name {}", addr, name),
            Err(_) => error!("TRACKER: Error inserting/updating the tracker"),
        }
    }

    /// Flush all presence entries that are older than the time specified in the argument
    pub fn flush(&mut self) {
        let horizon = Instant::now() - Duration::from_secs(60);
        self.souls.retain(|k, v| {
            if v.last_seen > horizon {
                true
            } else {
                info!(
                    "TRACKER: Removing {} with last presence at {:?}",
                    k, v.last_seen
                );
                false
            }
        })
    }
}
