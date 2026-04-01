use crate::operations::resize::ResizeAlgorithm;

#[derive(Clone)]
pub struct EncodingConfig {
    pub avif_speed: u8,
    pub avif_quality: u8,
    pub jpeg_quality: u8,
    pub webp_quality: f32,
    pub png_compression_level: u8,
    pub resize_algorithm: ResizeAlgorithm,
    pub root_path: String,
    pub strip_path: Option<String>,
    pub fallback_image_url: Option<String>,
    pub fallback_image_max_size: usize,
}

impl EncodingConfig {
    pub fn from_env() -> Self {
        Self {
            avif_speed: std::env::var("IMAGE_PROXY_AVIF_SPEED")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(7),
            avif_quality: std::env::var("IMAGE_PROXY_AVIF_QUALITY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(75),
            jpeg_quality: std::env::var("IMAGE_PROXY_JPEG_QUALITY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(75),
            webp_quality: std::env::var("IMAGE_PROXY_WEBP_QUALITY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(75.0),
            png_compression_level: std::env::var("IMAGE_PROXY_PNG_COMPRESSION_LEVEL")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(6),
            resize_algorithm: std::env::var("IMAGE_PROXY_RESIZE_ALGORITHM")
                .ok()
                .and_then(|s| ResizeAlgorithm::from_str(&s))
                .unwrap_or(ResizeAlgorithm::Auto),
            root_path: std::env::var("IMAGE_PROXY_ROOT_PATH")
                .unwrap_or_else(|_| "/app/data".to_string()),
            strip_path: std::env::var("IMAGE_PROXY_STRIP_PATH").ok(),
            fallback_image_url: std::env::var("IMAGE_PROXY_FALLBACK_IMAGE_URL").ok(),
            fallback_image_max_size: std::env::var("IMAGE_PROXY_FALLBACK_IMAGE_MAX_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5 * 1024 * 1024), // Default to 5 MB
        }
    }
}
