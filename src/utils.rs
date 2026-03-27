use std::path::Path;

use crate::config::EncodingConfig;

pub fn mime_type_for_format(format: Option<&str>) -> &'static str {
    match format {
        Some("avif") => "image/avif",
        Some("jpeg") | Some("jpg") => "image/jpeg",
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    }
}
pub fn convert_bytes_to_readable_size(bytes: u64) -> String {
    let units = ["B", "KiB", "MiB", "GiB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < units.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, units[unit_index])
}

#[tracing::instrument(level = "debug")]
pub fn load_bytes_from_disk(path: &Path) -> std::io::Result<Vec<u8>> {
    std::fs::read(path)
}

// TODO: rework into tagged enum to return HTTP response from actix-web or the data as bytes
#[tracing::instrument(level = "debug", skip(config))]
pub async fn load_fallback_image_from_url(
    partial_url: &str,
    config: &EncodingConfig,
) -> anyhow::Result<Vec<u8>> {
    let url = match &config.fallback_image_url {
        Some(base_url) => format!("{}{}", base_url, partial_url),
        None => partial_url.to_string(),
    };

    let response = reqwest::get(&url).await?;
    if !response.status().is_success() {
        anyhow::bail!("Failed to fetch fallback image: HTTP {}", response.status());
    }
    let bytes = response.bytes().await?;
    Ok(bytes.to_vec())
}
