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
