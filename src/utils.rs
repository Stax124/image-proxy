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

#[tracing::instrument(level = "debug")]
pub async fn load_bytes_from_disk(path: &Path) -> std::io::Result<Vec<u8>> {
    tokio::fs::read(path).await
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
