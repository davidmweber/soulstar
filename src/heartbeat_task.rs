use embassy_time::{Duration, Ticker};
use smart_leds::{brightness, RGB8};
use crate::colour::set_brightness;
use crate::led_driver::LedDriver1;
use crate::soul_config;

#[embassy_executor::task]
pub async fn heartbeat_task(led: &'static mut LedDriver1) {
    let mut animation = Ticker::every(Duration::from_millis(100));
    let colour = RGB8::from(soul_config::COLOUR);
    led.buffer[0] = colour;
    led.update_string();
    let mut brightness: i16 = 128;
    let mut up = false;
    loop {
        animation.next().await;
        if up {
            brightness = brightness + 32;
            if brightness > 128 {
                brightness = 0;
                up = false;
            }
        } else {
            brightness = brightness - 32;
            if brightness < 128 {
                brightness = 0;
                up = true;
            }
        }
        led.buffer[0] = set_brightness(brightness as u8, colour);
        led.update_string();
        
    }
} 