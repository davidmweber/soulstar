use embassy_time::{Duration, Timer};
use esp_hal::gpio::Input;

/// Simple debounced button press detection
pub async fn wait_for_press(button: &mut Input<'_>) {
    button.wait_for_rising_edge().await;
    Timer::after(Duration::from_millis(100)).await; // debounce
}
