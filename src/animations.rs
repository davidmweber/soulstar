use embassy_time::Instant;
use smart_leds::RGB8;
use crate::colour::set_brightness;
use crate::led_driver::LedBuffer;

pub enum AnimationType<'a> {
    Sparkle(SparkleAnimation<'a>),
}

pub trait Interruptable {
    /// If this is true then the animation is interruptable before its iterator returns None
    /// If a new soul arrives, we want it to sparkle for a few seconds and not be interrupted
    /// by a new arrival. Those can sit in the queue until this one is done. Be careful here
    /// as this could block all future animations sitting in the queue.
    fn is_interruptable(&self) -> bool;
}

pub trait Animation: Iterator<Item = LedBuffer> + Interruptable {}

/// Takes one colour and generates a random brightness up to the maximum brightness
/// specified. It will continue to return `Some(buffer)` until the `count` variable
/// drops to zero then it will return None.
pub struct SparkleAnimation<'a> {
    /// The colour to sparkle
    color: RGB8,
    /// Maximum brightness to sparkle to
    brightness: u8,
    /// The number of iterations it will run through before terminating the iterator
    count: usize,
    /// Our buffer containing the animation. It is public so you can just tell the LED driver
    /// to write it to the string
    pub led: &'a  LedBuffer,
    /// Random number generator for the sparkle effect
    rng: fastrand::Rng,
    /// Set this to true if the display manager is allowed to interrupt the animation
    interruptable: bool
}

impl<'a> Iterator for SparkleAnimation<'a> {
    type Item = LedBuffer;

    fn next(&mut self) -> Option<&'a Self::Item> {
        if self.count > 0 {
            for mut led in self.led.iter_mut() {
                let b = self.rng.u8(0..self.brightness);
                led = &mut set_brightness(b, self.color);
            }
            self.count -= 1;
            Some(self.led)
        } else {
            None
        }
    }
}

impl<'a> Interruptable for SparkleAnimation<'a> {
    fn is_interruptable(&self) -> bool {
        self.interruptable
    }
}

impl<'a> SparkleAnimation<'a> {
    fn new(color: RGB8, count: usize, brightness: u8, interruptable: bool) -> Self {
        let seed = Instant::now().as_ticks();
        Self {
            color,
            brightness,
            count,
            led: LedBuffer::default(),
            rng: fastrand::Rng::with_seed(seed),
            interruptable
        }
    }
}