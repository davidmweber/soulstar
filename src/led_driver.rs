use crate::configuration::LED_STRING_SIZE;
use esp_hal::Blocking;
use esp_hal::gpio::interconnect::PeripheralOutput;
use esp_hal::peripherals::RMT;
use esp_hal::rmt::{Channel, Rmt, TxChannel};
use esp_hal::time::Rate;
use esp_hal_smartled::{SmartLedsAdapter, smart_led_buffer};
use smart_leds::{RGB8, SmartLedsWrite};

/// The size of the LED strip we are driving.
const LED_INTERNAL_BUF_LEN: usize = LED_STRING_SIZE * 24 + 1;

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
        Self { led }
    }
}

impl<const C: u8> LedDriver<C>
where
    Channel<Blocking, C>: TxChannel,
{
    /// Update the contents of the buffer to the LED string, applying gamma correction and brightness.
    ///
    /// This must be called every time you want to propagate changes you have made to the string to
    /// the actual LED devices. This is not done automatically as you may want to do multiple changes
    /// before updating the display.
    ///
    /// Note that the update is a blocking operation, but it is quick enough for current needs.
    /// If non-blocking RMT setup becomes available, this will be changed to async.
    ///
    /// # Parameters
    /// * `led_buffer` - Buffer containing LED values to write to the string
    /// * `brightness` - Global brightness level from 0 (off) to 255 (max brightness)
    pub fn update_from_buffer(&mut self, led_buffer: &mut LedBuffer, brightness: u8) {
        let source = *led_buffer;
        let adjust_iter = smart_leds::brightness(smart_leds::gamma(source.iter().cloned()), brightness);
        for (pix, corrected) in led_buffer.iter_mut().zip(adjust_iter) {
            *pix = corrected;
        }
        self.led.write(*led_buffer).expect("Failed to update LED driver");
    }

    /// Switches all the LEDS off
    #[allow(unused)]
    pub fn all_off(&mut self) {
        self.update_from_buffer(&mut LedBuffer::default(), 0);
    }

    /// Switches all the LEDS to white at the specified brightness.
    ///
    /// # Parameters
    /// * `brightness` - The brightness level to set all LEDs to, from 0 (off) to 255 (full brightness)
    pub fn torch(&mut self, brightness: u8) {
        let mut b = LedBuffer::default();
        b.fill(RGB8 { r: 255, g: 255, b: 255 });
        self.update_from_buffer(&mut b, brightness);
    }
}
