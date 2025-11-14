use crate::utils::clip_min;

#[derive(Clone, Copy)]
pub enum Direction {
    Up,
    Down,
}

/// Throbber state
/// 
/// A throbber will slowly change its brightnes levels from `min` to 255 and back
/// each time the `next()` method is called. You can use the brightness to modulate
/// a LED colour before writing it to the led buffer for display
#[derive(Clone, Copy)]
pub struct Throbber {
    brightness: i16,
    direction: Direction,
    step: i16,
    min: u8,
    once: bool,
    done: bool,
}

impl Throbber {
    /// Create a throbber.
    ///
    /// # Parameters
    /// * `step` - Amount to change brightness by in each iteration
    /// * `min` - Minimum brightness value to not go below
    /// * `once` - Throb just once, ending when the brightness on  the Down direction reaches [min]
    #[allow(unused)]
    pub fn new(step: u8, min: u8, once: bool) -> Self {
        Self {
            brightness: min as i16,
            direction: Direction::Up,
            step: step as i16,
            min,
            once,
            done: false,
        }
    }

    /// Create a throbber that throbs once
    /// This throbber will start and end at zero, increment its brightness by the argument then
    /// decrement until it reaches zero brightness again. At this point, it will return None
    ///
    /// # Parameters
    /// * `step` - The size of the increment in steps. It must be less than 255
    #[allow(unused)]
    pub fn new_once(step: u8) -> Self {
        Self {
            brightness: 0,
            direction: Direction::Up,
            step: step as i16,
            min: 0,
            once:true,
            done: false,
        }
    }
    
    // Advances the steps by some fixed number so you can start the throbber
    // at some brightness other than min. I know this is lazy...
    pub fn advance(&mut self, steps: u8) {
        for _ in 0..steps { self.next();}
    }
}

impl Iterator for Throbber {
    type Item = u8;

    /// Next brightness value for this throbber
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        match self.direction {
            Direction::Up => {
                self.brightness = self.brightness + self.step;
                if self.brightness >= 255 {
                    self.direction = Direction::Down;
                    self.brightness = 255;
                }
            }
            Direction::Down => {
                self.brightness = self.brightness - self.step;
                if self.brightness < self.min as i16 {
                    // If we throb once, terminate after we hit the bottom of the cycle
                    if self.once {
                        self.done = true;
                    }
                    self.direction = Direction::Up;
                    self.brightness = self.min as i16;
                }
            }
        };
        Some(clip_min(self.brightness, self.min))
    }
}

/*
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn if_it_throbs_forever() {
        let mut t = Throbber::new(16, 8, false);
        let mut count = 0;
        let mut max_brightness = 0;
        let mut min_brightness = 255;
        while let Some(b) = t.next() {
            count += 1;
            max_brightness = max_brightness.max(b);
            min_brightness = min_brightness.min(b);
            if count > 1024 {
                break;
            }
        }
        assert_eq!(min_brightness, 8);
        assert_eq!(max_brightness, 255);
    }

    #[test]
    pub fn if_it_throbs_once() {
        let mut t = Throbber::new_once(16);
        // Iterate enough steps to hit the top at least once.
        let mut max_brightness = 0;
        let mut last_brightness = 100;
        let mut count = 0;
        while let Some(b) = t.next() {
            count += 1;
            max_brightness = max_brightness.max(b);
            last_brightness = b;
            if count > 1024 {
                break;
            }
        }
        assert_eq!(last_brightness, 0); // Throb once must only terminate after it goes dark
        assert_eq!(count, 32);
        assert_eq!(max_brightness, 255);
    }
}
*/