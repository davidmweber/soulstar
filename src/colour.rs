use smart_leds::RGB8;

#[allow(unused)]
pub fn set_brightness(brightness: u8, pixel: RGB8) -> RGB8 {
    if brightness == 0 {
        return RGB8::default();
    }
    if brightness == 255 {
        return pixel;
    }
    // Use u16 for the multiplication to avoid overflow before the division.
    let r = ((pixel.r as u16 * brightness as u16) / 255) as u8;
    let g = ((pixel.g as u16 * brightness as u16) / 255) as u8;
    let b = ((pixel.b as u16 * brightness as u16) / 255) as u8;

    RGB8::new(r, g, b)
}

#[allow(unused)]
fn clip(v: i16) -> u8 {
    if v < 0 {
        0
    } else if v > 255 {
        255
    } else {
        v as u8
    }
}

#[allow(unused)]
pub fn adjust_brightness_for_rssi(colour: RGB8, rssi: i8, brightness: u8) -> RGB8 {
    // Map -100 -> -40 dBm to a scale of 0-128
    let brightness = ((brightness as i16) * (rssi as i16 + 100) * 3) / 255;
    set_brightness(clip(brightness), colour)
}
