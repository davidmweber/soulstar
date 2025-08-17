//! Provide a mechanism to "throb" the brightness of one or more LEDs

use crate::throbber::Option::Some;
use core::iter::Iterator;
use core::option::Option;
use embassy_time::Instant;
use crate::colour::{clip, clip_min};

pub enum Direction {
    Up, Down
}

pub struct Throbber {
    brightness: u8,
    direction: Direction,
    step: i16,
    min: u8
}

impl Throbber {
    /// Create a throbber. 
    ///
    /// # Parameters
    /// * `brightness` - Initial brightness value (0-255)
    /// * `direction` - Initial direction of brightness change (Up or Down)
    /// * `step` - Amount to change brightness by in each iteration
    /// * `min` - Minimum brightness value to not go below
    #[allow(unused)]
    pub fn new(brightness: u8, direction: Direction, step: u8, min: u8) -> Self {
        Self {
            brightness, direction, step: step as i16, min
        }
    }

    /// Create a throbber starting at a random brightness and vary it with a random step in a 
    /// random direction.
    ///
    /// # Parameters
    /// * `min` - Minimum brightness value to not go below
    #[allow(unused)]
    pub fn new_random(min: u8) -> Self {
        let seed = Instant::now().as_ticks();
        let mut rng = fastrand::Rng::with_seed(seed);
        Self {
            brightness: rng.u8(min..),
            direction: if rng.bool() { Direction::Up } else { Direction::Down },
            step: rng.i16(8..64),
            min
        }
    }
}

impl Iterator for Throbber {
    type Item = u8;

    /// Next brightness value for this throbber
    fn next(&mut self) -> Option<Self::Item> {
        match self.direction {
            Direction::Up => {
                self.brightness = clip(self.brightness as i16 + self.step);
                if self.brightness == 255  {
                    self.direction = Direction::Down;
                }
            },
            Direction::Down => {
                self.brightness = clip_min(self.brightness as i16 - self.step, self.min);
                if self.brightness == self.min {
                    self.direction = Direction::Up;
                }
            }
        };
        Some(self.brightness)
    }
}
