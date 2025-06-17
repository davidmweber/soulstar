use embassy_sync::blocking_mutex::CriticalSectionMutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use smart_leds::RGB8;
use embassy_sync::channel::Channel;
use log::info;
use static_cell::StaticCell;


/// Manage the display state by sending it messages of this type. If anyone asks why I like Rust,
/// this is one of the many reasons
enum DisplayState {
    Stop,
    Start,
    Colour(RGB8)
}
type ControlChannel = Channel<CriticalSectionRawMutex, DisplayState, 3>;

/// Communicate with the display task using this channel and the DisplayState enum
static DISPLAY_CHANNEL: StaticCell<ControlChannel> = StaticCell::new();


#[embassy_executor::task]
async fn display_task(channel: &'static ControlChannel) {
    info!("DISPLAY_TASK: Task started. Waiting for messages...");
    loop {
        let msg = channel.receive().await;
        info!("DISPLAY_TASK: Got message: {}", msg);
    }
}