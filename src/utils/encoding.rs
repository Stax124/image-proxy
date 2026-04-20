pub fn jxl_encoder_speed_from_int(speed: u8) -> jpegxl_rs::encode::EncoderSpeed {
    match speed {
        1 => jpegxl_rs::encode::EncoderSpeed::Lightning,
        2 => jpegxl_rs::encode::EncoderSpeed::Thunder,
        3 => jpegxl_rs::encode::EncoderSpeed::Falcon,
        4 => jpegxl_rs::encode::EncoderSpeed::Cheetah,
        5 => jpegxl_rs::encode::EncoderSpeed::Hare,
        6 => jpegxl_rs::encode::EncoderSpeed::Wombat,
        7 => jpegxl_rs::encode::EncoderSpeed::Squirrel,
        8 => jpegxl_rs::encode::EncoderSpeed::Kitten,
        9 => jpegxl_rs::encode::EncoderSpeed::Tortoise,
        10 => jpegxl_rs::encode::EncoderSpeed::Glacier,
        _ => jpegxl_rs::encode::EncoderSpeed::Squirrel, // Default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jxl_encoder_speed_known_values() {
        use jpegxl_rs::encode::EncoderSpeed;
        assert!(matches!(
            jxl_encoder_speed_from_int(1),
            EncoderSpeed::Lightning
        ));
        assert!(matches!(
            jxl_encoder_speed_from_int(6),
            EncoderSpeed::Wombat
        ));
        assert!(matches!(
            jxl_encoder_speed_from_int(9),
            EncoderSpeed::Tortoise
        ));
        assert!(matches!(
            jxl_encoder_speed_from_int(10),
            EncoderSpeed::Glacier
        ));
    }

    #[test]
    fn jxl_encoder_speed_out_of_range_defaults_to_squirrel() {
        use jpegxl_rs::encode::EncoderSpeed;
        assert!(matches!(
            jxl_encoder_speed_from_int(0),
            EncoderSpeed::Squirrel
        ));
        assert!(matches!(
            jxl_encoder_speed_from_int(11),
            EncoderSpeed::Squirrel
        ));
    }
}
