use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum PathValidationError {
    MissingExtension,
    UnsupportedFormat(String),
}

pub fn sanitize_and_validate_path(
    filename: &str,
    strip_path: Option<&str>,
    root_path: &str,
    supported_formats: &[&str],
) -> Result<(PathBuf, PathBuf, String), PathValidationError> {
    // Strip the specified path from the filename if configured
    let filename = if let Some(strip_path) = strip_path {
        filename
            .strip_prefix(strip_path)
            .unwrap_or(filename)
            .to_string()
    } else {
        filename.to_string()
    };

    // Sanitize the path to prevent directory traversal
    let sanitized_path = Path::new(&filename)
        .components()
        .filter(|comp| matches!(comp, std::path::Component::Normal(_)))
        .fold(Path::new("").to_path_buf(), |acc, comp| {
            acc.join(comp.as_os_str())
        });

    // Join the sanitized path with the root path to get the final file path
    let sanitized_disk_path = Path::new(root_path).join(&sanitized_path);

    // Check if the extension is valid and supported
    let file_ext = sanitized_disk_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase());
    let Some(ext) = file_ext else {
        return Err(PathValidationError::MissingExtension);
    };
    if !supported_formats.contains(&ext.as_str()) {
        return Err(PathValidationError::UnsupportedFormat(ext));
    }

    Ok((sanitized_path, sanitized_disk_path, ext))
}

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

    const FORMATS: &[&str] = &["avif", "jpeg", "jpg", "png", "webp"];

    #[test]
    fn sanitize_valid_jpeg_path() {
        let (sanitized, disk, ext) =
            sanitize_and_validate_path("photo.jpeg", None, "/data", FORMATS).unwrap();
        assert_eq!(sanitized, PathBuf::from("photo.jpeg"));
        assert_eq!(disk, PathBuf::from("/data/photo.jpeg"));
        assert_eq!(ext, "jpeg");
    }

    #[test]
    fn sanitize_strips_prefix() {
        let (sanitized, disk, ext) =
            sanitize_and_validate_path("prefix/photo.png", Some("prefix/"), "/data", FORMATS)
                .unwrap();
        assert_eq!(sanitized, PathBuf::from("photo.png"));
        assert_eq!(disk, PathBuf::from("/data/photo.png"));
        assert_eq!(ext, "png");
    }

    #[test]
    fn sanitize_prevents_directory_traversal() {
        let (sanitized, disk, _) =
            sanitize_and_validate_path("../secret.jpeg", None, "/data", FORMATS).unwrap();
        // Traversal component stripped
        assert_eq!(sanitized, PathBuf::from("secret.jpeg"));
        assert_eq!(disk, PathBuf::from("/data/secret.jpeg"));
    }

    #[test]
    fn sanitize_missing_extension_error() {
        let result = sanitize_and_validate_path("noext", None, "/data", FORMATS);
        assert!(matches!(result, Err(PathValidationError::MissingExtension)));
    }

    #[test]
    fn sanitize_unsupported_format_error() {
        let result = sanitize_and_validate_path("file.bmp", None, "/data", FORMATS);
        assert!(matches!(
            result,
            Err(PathValidationError::UnsupportedFormat(ref e)) if e == "bmp"
        ));
    }

    #[test]
    fn sanitize_nested_path() {
        let (sanitized, disk, ext) =
            sanitize_and_validate_path("a/b/c/photo.webp", None, "/root", FORMATS).unwrap();
        assert_eq!(sanitized, PathBuf::from("a/b/c/photo.webp"));
        assert_eq!(disk, PathBuf::from("/root/a/b/c/photo.webp"));
        assert_eq!(ext, "webp");
    }

    #[test]
    fn sanitize_case_insensitive_extension() {
        // Extension is lowercased, so "JPG" becomes "jpg" which is in FORMATS
        let (_, _, ext) = sanitize_and_validate_path("photo.JPG", None, "/data", FORMATS).unwrap();
        assert_eq!(ext, "jpg");
    }
}
