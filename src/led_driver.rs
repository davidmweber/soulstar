use esp_hal::Blocking;
use esp_hal::gpio::interconnect::PeripheralOutput;
use esp_hal::peripherals::RMT;
use esp_hal::rmt::{Channel, Rmt, TxChannel};
use esp_hal::time::Rate;
use esp_hal_smartled::{SmartLedsAdapter, smart_led_buffer};
use smart_leds::{RGB8, SmartLedsWrite};
use crate::configuration::LED_STRING_SIZE;

/// The size of the LED strip we are driving.
const LED_INTERNAL_BUF_LEN: usize =  LED_STRING_SIZE * 24 + 1;

pub type LedDriver0 = LedDriver<0>;

/// Convenience type so we speak the same language when dealing with animations etc.
pub type LedBuffer = [RGB8; LED_STRING_SIZE];

/// Holds the state needed to drive the LED strip
pub struct LedDriver<const C: u8>
where
    Channel<Blocking, C>: TxChannel,
{
    /// Driver for the led array. We have to size it here to exactly what we will get back from
    /// the `SmartLedsAdapter::new()` function when we set up the driver below
    led: SmartLedsAdapter<Channel<Blocking, C>, LED_INTERNAL_BUF_LEN>,
    /// This is the backing buffer into which we write the pattern we want
    pub buffer: LedBuffer,
}

impl LedDriver<0> {
    /// Create a new driver for the LED string. It requires an RMT peripheral
    /// device and a GPIO pin. It is hardwired to use channel 0 for the RMT device
    ///
    /// # Parameters
    /// * `rmt` - The RMT peripheral device to use for driving the LED strip
    /// * `pin` - The GPIO pin to which the LED strip is connected
    /// * `channel` -  The RMT device channel to use
    pub fn new<'a>(rmt: RMT, pin: impl PeripheralOutput<'a>) -> Self {
        let frequency = Rate::from_mhz(80);
        let rmt_dev = Rmt::new(rmt, frequency).expect("Failed to initialize RMT0");
        let led = SmartLedsAdapter::new(rmt_dev.channel0, pin, smart_led_buffer!(LED_STRING_SIZE));
        let buffer: LedBuffer = [Default::default(); LED_STRING_SIZE];
        LedDriver { led, buffer }
    }
}

impl<const C: u8> LedDriver<C>
where
    Channel<Blocking, C>: TxChannel,
{

    /// Update the contents of the buffer to the LED string. This must be called
    /// every time you want to propagate changes you have made to the string to the actual led
    /// devices. This is not done automatically as you may want to do multiple changes to what gets
    /// displayed before you update. Note that the update is a blocking operation but it is quick
    /// enough for us. If I can figure out a non-blocking RMT setup, I will change this to async.
    pub fn update_string(&mut self) {
        self.led.write(self.buffer).expect("Failed to update LED driver");
    }

    /// Rotate the whole array one step to the left
    #[allow(unused)]
    pub fn rotate_left(&mut self) {
        self.buffer.rotate_left(1);
    }

    /// Rotate the whole array one step to the right
    #[allow(unused)]
    pub fn rotate_right(&mut self) {
        self.buffer.rotate_right(1)
    }

    /// Switches all the LEDS off
    #[allow(unused)]
    pub fn all_off(&mut self) {
        self.buffer.fill(RGB8 { r: 0, g: 0, b: 0 });
    }

    /// Switches all the LEDS to white at the specified brightness.
    ///
    /// # Parameters
    /// * `brightness` - The brightness level to set all LEDs to, from 0 (off) to 255 (full brightness)
    #[allow(unused)]
    pub fn torch(&mut self, brightness: u8) {
        self.buffer.fill(RGB8 {
            r: brightness,
            g: brightness,
            b: brightness,
        });
    }
}
