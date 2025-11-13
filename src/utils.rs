/// Arb things I did not know where else to put
#[allow(unused)]
pub fn clip(v: i16) -> u8 {
    if v < 0 {
        0
    } else if v > 255 {
        255
    } else {
        v as u8
    }
}

/// Clip to a minimum value
pub fn clip_min(v: i16, min: u8) -> u8 {
    if v < min as i16 {
        min
    } else if v > 255 {
        255
    } else {
        v as u8
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn if_it_clips() {
        assert_eq!(clip(-1), 0);
        assert_eq!(clip(1), 1);
        assert_eq!(clip(255), 255);
        assert_eq!(clip(256), 255);
    }

    #[test]
    pub fn if_it_limits() {
        assert_eq!(clip_min(128, 10), 128);
        assert_eq!(clip_min(5, 10), 10);
        assert_eq!(clip_min(256, 10), 255);
        assert_eq!(clip_min(255, 10), 255);
    }
}
