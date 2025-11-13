use crate::configuration::LED_STRING_SIZE;
use esp_hal::Async;
use esp_hal::gpio::interconnect::PeripheralOutput;
use esp_hal::rmt::PulseCode;
use esp_hal_smartled::{SmartLedsAdapterAsync, buffer_size_async};
use smart_leds::{RGB8, SmartLedsWriteAsync};
use static_cell::StaticCell;

/// We must know what the LED TX buffer size is as a constant for the types involved here
const LED_INTERNAL_BUF_LEN: usize = buffer_size_async(LED_STRING_SIZE);

/// Convenience type so we speak the same language when dealing with animations etc.
pub type LedBuffer = [RGB8; LED_STRING_SIZE];

static RMT_BUFFER: StaticCell<[PulseCode; buffer_size_async(LED_STRING_SIZE)]> = StaticCell::new();

/// Holds the state needed to drive the LED strip
pub struct LedDriver<'a> {
    /// Driver for the led array. We have to size it here to exactly what we will get back from
    /// the `SmartLedsAdapterAsync::new()` function when we set up the driver below
    led: SmartLedsAdapterAsync<'a, LED_INTERNAL_BUF_LEN>,
}

impl<'a> LedDriver<'a> {
    /// Create a new driver for the LED string. It requires an RMT peripheral
    /// device and a GPIO pin. It is hardwired to use channel 0 for the RMT device.
    /// See [this example](https://github.com/cmumford/esp-hal-community/blob/channel-creator/esp-hal-smartled/examples/hello_rgb_async.rs)
    /// for the async API.
    ///
    /// # Parameters
    /// * `rmt` - The RMT peripheral device to use for driving the LED strip
    /// * `pin` - The GPIO pin to which the LED strip is connected
    pub fn new(rmt: esp_hal::rmt::Rmt<'a, Async>, pin: impl PeripheralOutput<'a>) -> Self {
        //
        let channel = rmt.channel0;
        let buffer = RMT_BUFFER.init([PulseCode::default(); buffer_size_async(LED_STRING_SIZE)]);
        let led = SmartLedsAdapterAsync::new(channel, pin, buffer);
        Self { led }
    }
}

impl<'a> LedDriver<'a> {
    /// Update the contents of the buffer to the LED string, applying gamma correction and brightness.
    ///
    /// This must be called every time you want to propagate changes you have made to the string to
    /// the actual LED devices. This is not done automatically as you may want to do multiple changes
    /// before updating the display.
    ///
    /// # Parameters
    /// * `led_buffer` - Buffer containing LED values to write to the string
    /// * `brightness` - Global brightness level from 0 (off) to 255 (max brightness)
    pub async fn update_from_buffer(&mut self, led_buffer: &mut LedBuffer, brightness: u8) {
        let source = *led_buffer;
        let adjust_iter = smart_leds::brightness(smart_leds::gamma(source.iter().cloned()), brightness);
        for (pix, corrected) in led_buffer.iter_mut().zip(adjust_iter) {
            *pix = corrected;
        }
        self.led.write(*led_buffer).await.expect("Failed to update LED driver");
    }

    /// Switches all the LEDS off
    #[allow(unused)]
    pub async fn all_off(&mut self) {
        self.update_from_buffer(&mut LedBuffer::default(), 0).await;
    }

    /// Switches all the LEDS to white at the specified brightness.
    ///
    /// # Parameters
    /// * `brightness` - The brightness level to set all LEDs to, from 0 (off) to 255 (full brightness)
    pub async fn torch(&mut self, brightness: u8) {
        let mut b = LedBuffer::default();
        b.fill(RGB8 { r: 255, g: 255, b: 255 });
        self.update_from_buffer(&mut b, brightness).await;
    }
}
