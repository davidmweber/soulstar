use esp_hal::Blocking;
use esp_hal::gpio::interconnect::PeripheralOutput;
use esp_hal::peripherals::RMT;
use esp_hal::rmt::{Channel, Rmt};
use esp_hal::time::Rate;
use esp_hal_smartled::{SmartLedsAdapter, smart_led_buffer};
use smart_leds::{RGB8, SmartLedsWrite};

/// The size of the LED strip we are driving.
const STRIP_SIZE: usize = 24;

/// Holds the state needed to drive the LED strip
pub struct LedDriver {
    /// Driver for the led array. We have to size it here to exactly what we will get back from
    /// the `SmartLedsAdapter::new()` function when we set up the driver below
    led: SmartLedsAdapter<Channel<Blocking, 0>, { STRIP_SIZE * 24 + 1 }>,
    /// This is the backing buffer into which we write the pattern we want
    pub buffer: [RGB8; STRIP_SIZE],
}

impl LedDriver {
    /// Create a new driver for the LED string. It requires an RMT peripheral
    /// device and a GPIO pin. It is hardwired to use channel 0 for the RMT device
    pub fn new<'a>(rmt: RMT, pin: impl PeripheralOutput<'a>) -> Self {
        let led = {
            let frequency = Rate::from_mhz(80);
            let rmt_dev = Rmt::new(rmt, frequency).expect("Failed to initialize RMT0");
            SmartLedsAdapter::new(rmt_dev.channel0, pin, smart_led_buffer!(STRIP_SIZE))
        };
        let buffer: [RGB8; STRIP_SIZE] = [Default::default(); STRIP_SIZE];
        LedDriver { led, buffer }
    }

    /// Update the contents of the buffer to the LED string. This must be called
    /// every time you want to propagate changes you have made to the string to the actual led
    /// devices. This is not done automatically as you may want to do multiple changes to what gets
    /// displayed before you update. Note that the update is a blocking operation but it is quick
    /// enough for us. If I can figure out a non-blocking RMT setup, I will change this to async.
    pub fn update_string(&mut self) {
        self.led
            .write(self.buffer)
            .expect("Failed to update LED driver");
    }

    /// Rotate the whole array one step to the left
    #[allow(unused)]
    pub fn rotate_left(&mut self) {
        self.buffer.rotate_left(1);
    }

    /// Rotate the whole array one step to the right
    #[allow(unused)]
    pub fn rotate_right(&mut self) {
        self.buffer.rotate_left(1)
    }
}
