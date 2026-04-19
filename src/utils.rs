use std::path::Path;

pub fn mime_type_for_format(format: Option<&str>) -> &'static str {
    match format {
        Some("avif") => "image/avif",
        Some("jpeg") | Some("jpg") => "image/jpeg",
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("jxl") => "image/jxl",
        _ => "application/octet-stream",
    }
}

#[tracing::instrument(level = "debug")]
pub async fn load_bytes_from_disk(path: &Path) -> std::io::Result<Vec<u8>> {
    tokio::fs::read(path).await
}

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
    fn mime_type_avif() {
        assert_eq!(mime_type_for_format(Some("avif")), "image/avif");
    }

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

    #[test]
    fn mime_type_unknown_format() {
        assert_eq!(
            mime_type_for_format(Some("bmp")),
            "application/octet-stream"
        );
        assert_eq!(
            mime_type_for_format(Some("tiff")),
            "application/octet-stream"
        );
    }

    #[test]
    fn mime_type_none() {
        assert_eq!(mime_type_for_format(None), "application/octet-stream");
    }

    #[tokio::test]
    async fn load_bytes_from_disk_existing_file() {
        let dir = std::env::temp_dir().join("image_proxy_test_load");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.bin");
        std::fs::write(&path, b"hello").unwrap();

        let bytes = load_bytes_from_disk(&path).await.unwrap();
        assert_eq!(bytes, b"hello");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn load_bytes_from_disk_missing_file() {
        let result = load_bytes_from_disk(Path::new("/tmp/nonexistent_image_proxy_test")).await;
        assert!(result.is_err());
    }
}
