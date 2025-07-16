//! Animations module provides different LED animation patterns for the Soul Star device.
//!
//! This module contains implementations for various LED animations including:
//! - Sparkle animations that create random brightness variations of a single colour
//! - Presence animations that display and rotate colours representing visible souls

use crate::colour::set_brightness;
use crate::led_driver::LedBuffer;
use crate::tracker::VisibleSouls;
use defmt::{Format, Formatter, write};
use embassy_time::{Duration, Instant};
use smart_leds::RGB8;

/// Represents different types of animations that can be displayed on the LED strip
#[derive(Clone)]
pub enum Animation {
    /// Animation that creates a sparkling effect with random brightness variations
    Sparkle(SparkleAnimation),
    /// Animation that displays and rotates colours representing visible souls
    Presence(PresenceAnimation),
}

/// Checks if the given animation can be interrupted
///
/// # Arguments
/// * `anim` - Reference to the Animation to check
///
/// # Returns
/// True if the animation can be interrupted, false otherwise
pub fn is_interruptable(anim: &Animation) -> bool {
    match anim {
        Animation::Sparkle(s) => s.is_interruptable(),
        Animation::Presence(s) => s.is_interruptable(),
    }
}

/// Helper function to get the new buffer regardless of animation. This is because we cannot use
///  [dyn traits](https://doc.rust-lang.org/rust-by-example/trait/dyn.html) in a `no_std` without
/// setting up a heap. I know we do have an allocator, but I wanted this simple.
///
/// # Arguments
/// * `anim` - A mutable reference to the Animation enum that will generate the next buffer state
/// # Returns
/// The result of the iterator on the animation
pub fn next_buffer(anim: &mut Animation) -> Option<LedBuffer> {
    match anim {
        Animation::Sparkle(s) => s.next(),
        Animation::Presence(p) => p.next(),
    }
}

impl Format for Animation {
    fn format(&self, fmt: Formatter) {
        match self {
            Animation::Sparkle(_) => write!(fmt, "Sparkle"),
            Animation::Presence(_) => write!(fmt, "Presence"),
        }
    }
}

pub trait Interruptable {
    /// If this is true then the animation is interruptable before its iterator returns None
    /// If a new soul arrives, we want it to sparkle for a few seconds and not be interrupted
    /// by a new arrival. Those can sit in the queue until this one is done. Be careful here
    /// as this could block all future animations sitting in the queue.
    fn is_interruptable(&self) -> bool;
}

/// Takes one colour and generates a random brightness up to the maximum brightness
/// specified. It will continue to return `Some(buffer)` until the expiry time is reached
/// if one was specified
#[derive(Clone)]
pub struct SparkleAnimation {
    /// The colour to sparkle
    colour: RGB8,
    /// The system time at which the animation should expire. If it is None, the animation
    /// will run but will mark itself as interruptable.
    expires: Option<Instant>,
    /// Random number generator for the sparkle effect
    rng: fastrand::Rng,
}

impl Iterator for SparkleAnimation {
    type Item = LedBuffer;

    fn next(&mut self) -> Option<Self::Item> {
        let done = match self.expires {
            Some(exp) if Instant::now() < exp => false, // Have expiration but not expired so not done
            None => false,                              // No expiration is never done
            _ => true,                                  // All other cases are done
        };

        if !done {
            let mut buffer = LedBuffer::default();
            for led in buffer.iter_mut() {
                let b = self.rng.u8(0..255);
                *led = set_brightness(b, self.colour);
            }
            Some(buffer)
        } else {
            None
        }
    }
}

impl Interruptable for SparkleAnimation {
    fn is_interruptable(&self) -> bool {
        self.expires.is_none()
    }
}

impl SparkleAnimation {
    /// Creates a new SparkleAnimation instance that generates random brightness variations of a base colour
    ///
    /// # Arguments
    /// * `colour` - The base RGB colour to be used for the sparkle effect
    /// * `ttl` - Optional Duration that specifies how long the animation should run. None implies indefinitely
    ///
    /// Returns a new SparkleAnimation instance initialised with the current time as the RNG seed and
    /// the specified parameters. The animation will be interruptible if no ttl is provided
    pub(crate) fn new(colour: RGB8, ttl: Option<Duration>) -> Self {
        let seed = Instant::now().as_ticks();
        let expires = ttl.map(|t| Instant::now() + t);
        Self {
            colour,
            expires,
            rng: fastrand::Rng::with_seed(seed),
        }
    }
}

/// Animation that displays and rotates colours representing visible souls
///
/// This animation takes a collection of visible souls and their associated colours,
/// displays them on the LED strip, and rotates their positions over time. If the
/// number of souls in the presence list is zero then the animation will terminate.
#[derive(Clone)]
pub struct PresenceAnimation {
    /// Collection of currently visible souls and their colours
    souls: VisibleSouls,
    /// Current rotation index for the animation
    index: usize,
}

impl Iterator for PresenceAnimation {
    type Item = LedBuffer;
    fn next(&mut self) -> Option<Self::Item> {
        if self.souls.is_empty() {
            return None;
        }
        let mut buffer = LedBuffer::default();
        let mut idx = 0;
        #[allow(clippy::explicit_counter_loop)]
        for s in &self.souls {
            buffer[idx] = s.colour;
            idx += 1;
        }
        buffer.rotate_right(self.index);
        self.index = (self.index + 1) % buffer.len();
        Some(buffer)
    }
}

impl Interruptable for PresenceAnimation {
    /// Presence animations are always interruptable
    fn is_interruptable(&self) -> bool {
        true
    }
}

impl PresenceAnimation {
    pub fn new(souls: &VisibleSouls) -> Self {
        Self {
            souls: souls.clone(),
            index: 0,
        }
    }
}
