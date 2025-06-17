use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use smart_leds::RGB8;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Timer};
use log::info;
use crate::led_driver::LedDriver;

/// Manage the display state by sending it messages of this type. If anyone asks why I like Rust,
/// this is one of the many reasons
pub enum DisplayState {
    Stop,
    Start,
    Colour(RGB8)
}
pub type DisplayControlChannel = Channel<CriticalSectionRawMutex, DisplayState, 3>;



#[embassy_executor::task]
pub async fn display_task(channel: &'static DisplayControlChannel, led: &'static mut LedDriver) {
    info!("DISPLAY_TASK: Task started. Waiting for messages...");
    led.buffer[0] = RGB8::new(1, 0, 0);
    led.buffer[1] = RGB8::new(0, 5, 0);
    led.buffer[2] = RGB8::new(1, 0, 0);

    led.update_string();
    info!("Entering main loop");
    loop {
        Timer::after(Duration::from_millis(100)).await;
        led.rotate_left();
        led.update_string();
    }
}