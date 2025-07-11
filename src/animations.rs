use crate::colour::set_brightness;
use crate::led_driver::LedBuffer;
use defmt::info;
use embassy_time::{Duration, Instant};
use smart_leds::RGB8;

#[derive(Clone)]
pub enum Animation {
    Sparkle(SparkleAnimation),
}

pub trait Interruptable {
    /// If this is true then the animation is interruptable before its iterator returns None
    /// If a new soul arrives, we want it to sparkle for a few seconds and not be interrupted
    /// by a new arrival. Those can sit in the queue until this one is done. Be careful here
    /// as this could block all future animations sitting in the queue.
    fn is_interruptable(&self) -> bool;
}

//pub trait Animation: Iterator<Item = LedBuffer> + Interruptable {}

/// Takes one colour and generates a random brightness up to the maximum brightness
/// specified. It will continue to return `Some(buffer)` until the `count` variable
/// drops to zero, then it will return None.
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
}

impl SparkleAnimation {
    pub(crate) fn new(color: RGB8, ttl: Option<Duration>) -> Self {
        let seed = Instant::now().as_ticks();
        let expires = if let Some(t) =  ttl {
            Instant::now() + t
        } else {
            Instant::now() + Duration::from_secs(u64::MAX)
        };
        info!("SPARKLE: Starting Sparkle animation {} {}", Instant::now().as_ticks(), expires.as_ticks());
        Self {
            color,
            expires,
            rng: fastrand::Rng::with_seed(seed),
            interruptable: ttl.is_none(), // Allow interruptions if this animation does not have a timeout
        }
    }
}
