use embassy_time::Instant;
// Track presence messages
use heapless::FnvIndexMap;
use crate::display_task::PresenceMessage;

struct Tracker<const S: usize> {
    souls: FnvIndexMap<u8, PresenceMessage, S>
}


impl<const S: usize> Tracker<S> {
    /// Updates the tracker with the lastest presence messages
    fn update(&self, presence: PresenceMessage)  {
        let poop = Instant::now();
        
    }

}