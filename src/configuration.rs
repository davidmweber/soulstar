use trouble_host::prelude::TxPower;

/// The display animation update interval in milliseconds
pub const ANIMATION_UPDATE: u64 = 250;

/// If a soul has not been seen for more than this many seconds, they are flushed
/// from the presence list
pub const TRACKER_FLUSH_AGE: u64 = 15;

/// How long must the new soul arrival animation run in seconds
pub const NEW_SOUL_ANIMATION: u64 = 1;

/// Maximum number of souls to track. Must be a power of two because of the heapless crate
pub const MAX_SOULS_TRACKED: usize = 16;

/// Transmission power for the advertisement beacon. Generally, the bigger, the longer the range.
/// These are an enum in trouble_host and go up to Plus20dBM
pub const TX_POWER: TxPower = TxPower::ZerodBm;

/// A global company ID that we set here so we can filter beacons for only SoulStar devices
pub const COMPANY_ID: u16 = 0xBEEF;

/// The number of LEDs in the string we are driving
pub const LED_STRING_SIZE: usize = 24;
