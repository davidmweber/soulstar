use esp_hal::gpio::interconnect::PeripheralOutput;
use esp_hal::peripherals::RMT;
use esp_hal::rmt::{Channel, Rmt};
use esp_hal::time::Rate;
use esp_hal::Blocking;
use esp_hal_smartled::{smart_led_buffer, SmartLedsAdapter};
use smart_leds::{SmartLedsWrite, RGB};

const STRIP_SIZE: usize = 24;
const BUFFER_SIZE: usize = STRIP_SIZE * 24 + 1;
pub struct LedDriver {
    pub led: SmartLedsAdapter<Channel<Blocking, 0>, BUFFER_SIZE>,
    buffer: [RGB<u8>; STRIP_SIZE],
}

impl LedDriver {

    /// Create a new driver for the
    pub fn new<'a>(rmt: RMT, pin: impl PeripheralOutput<'a>) -> Self {
        let led = {
            let frequency = Rate::from_mhz(80);
            let rmt_dev = Rmt::new(rmt, frequency).expect("Failed to initialize RMT0");
            SmartLedsAdapter::new(rmt_dev.channel0, pin, smart_led_buffer!(STRIP_SIZE))
        };
        let mut buffer  :[RGB<u8>; STRIP_SIZE] = [Default::default(); STRIP_SIZE];
        buffer[0] = RGB::new(0, 127, 0);
        LedDriver { led, buffer }
    }

    pub fn update_string(mut self) -> () {
        self.led.write(self.buffer.into_iter()).expect("Failed to update LED driver");
    }
}
