#[derive(Clone)]
pub struct EncodingConfig {
    pub avif_speed: u8,
    pub avif_quality: u8,
    pub jpeg_quality: u8,
    pub webp_quality: f32,
    pub png_compression_level: u8,
    pub use_faster_resize: bool,
    pub root_path: String,
    pub strip_path: Option<String>,
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
            use_faster_resize: std::env::var("IMAGE_PROXY_USE_FASTER_RESIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(true),
            root_path: std::env::var("IMAGE_PROXY_ROOT_PATH")
                .unwrap_or_else(|_| "/app/data".to_string()),
            strip_path: std::env::var("IMAGE_PROXY_STRIP_PATH").ok(),
        }
    }
}
