use crate::colour::set_brightness;
use crate::led_driver::LedBuffer;
use defmt::info;
use embassy_time::{Duration, Instant};
use smart_leds::RGB8;

#[derive(Clone)]
pub enum Animation {
    Sparkle(SparkleAnimation),
    Torch(TorchAnimation),
}

pub trait Interruptable {
    /// If this is true then the animation is interruptable before its iterator returns None
    /// If a new soul arrives, we want it to sparkle for a few seconds and not be interrupted
    /// by a new arrival. Those can sit in the queue until this one is done. Be careful here
    /// as this could block all future animations sitting in the queue.
    fn is_interruptable(&self) -> bool;

    /// Returns true if the animation is static (as in it never updates). It means it can
    /// get called once with a single update for the LED string
    fn is_static(&self) -> bool;
}

//pub trait Animation: Iterator<Item = LedBuffer> + Interruptable {}

/// Takes one colour and generates a random brightness up to the maximum brightness
/// specified. It will continue to return `Some(buffer)` until the `count` variable
/// drops to zero then it will return None.
#[derive(Clone)]
pub struct SparkleAnimation {
    /// The colour to sparkle
    color: RGB8,
    /// The system time at which the animation should expire
    expires: Instant,
    /// Random number generator for the sparkle effect
    rng: fastrand::Rng,
    /// Set this to true if the display manager is allowed to interrupt the animation
    interruptable: bool,
}

impl Iterator for SparkleAnimation {
    type Item = LedBuffer;

    fn next(&mut self) -> Option<Self::Item> {
        if Instant::now() < self.expires {
            let mut buffer = LedBuffer::default();
            for led in buffer.iter_mut() {
                let b = self.rng.u8(0..255);
                *led = set_brightness(b, self.color);
            }
            Some(buffer)
        } else {
            None
        }
    }
}

impl Interruptable for SparkleAnimation {
    fn is_interruptable(&self) -> bool {
        self.interruptable
    }
    fn is_static(&self) -> bool {
        false
    }
}

impl SparkleAnimation {
    pub(crate) fn new(color: RGB8, ttl: Duration, interruptable: bool) -> Self {
        let seed = Instant::now().as_ticks();
        let expires = Instant::now() + ttl;
        info!("SPARKLE: Starting Sparkle animation {} {}", Instant::now().as_ticks(), expires.as_ticks());
        Self {
            color,
            expires,
            rng: fastrand::Rng::with_seed(seed),
            interruptable,
        }
    }
}

// Will just set the brightness
#[derive(Clone)]
pub struct TorchAnimation;

impl Iterator for TorchAnimation {
    type Item = LedBuffer;
    fn next(&mut self) -> Option<Self::Item> {
        let mut buffer = LedBuffer::default();
        for led in buffer.iter_mut() {
            // Torch is white..... Brighness is managed in the display driver
            *led = RGB8::new(255, 255, 255);
        }
        Some(buffer)
    }
}

impl Interruptable for TorchAnimation {
    fn is_interruptable(&self) -> bool {
        true
    }
    fn is_static(&self) -> bool {
        true
    }
}

struct SoulAnimation {}
