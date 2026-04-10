use std::path::Path;

pub fn mime_type_for_format(format: Option<&str>) -> &'static str {
    match format {
        Some("avif") => "image/avif",
        Some("jpeg") | Some("jpg") => "image/jpeg",
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    }
}

#[tracing::instrument(level = "debug")]
pub async fn load_bytes_from_disk(path: &Path) -> std::io::Result<Vec<u8>> {
    tokio::fs::read(path).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mime_type_avif() {
        assert_eq!(mime_type_for_format(Some("avif")), "image/avif");
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
