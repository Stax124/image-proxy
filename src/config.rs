use crate::operations::resize::ResizeAlgorithm;

#[derive(Clone, Debug)]
pub struct EncodingConfig {
    // JPEG encoding parameters
    pub jpeg_quality: u8,

    // PNG encoding parameters
    pub png_compression_level: u8,

    // AVIF encoding parameters
    pub avif_quality: u8,
    /// AVIF encoding speed (0-10); higher is faster but worse compression
    pub avif_speed: u8,

    // WebP encoding parameters
    pub webp_quality: u8,
    /// WebP effort level (0-6); higher is slower but better compression
    pub webp_effort: u8,

    // JPEG XL encoding parameters
    pub jxl_quality: u8,
    /// JPEG XL encoding speed (0-10); higher is faster but worse compression
    pub jxl_speed: u8,

    // Resizing parameters
    pub resize_algorithm: ResizeAlgorithm,

    // Path configuration
    pub root_path: String,
    pub strip_path: Option<String>,

    // Fallback image configuration
    pub fallback_image_url: Option<String>,
    pub fallback_image_max_size: usize,

    // Cache configuration
    /// Whether to enable caching at all
    pub enable_cache: bool,
    /// Memory cache size in bytes
    pub cache_memory_size: usize,
    /// Whether to enable disk caching
    pub enable_disk_cache: bool,
    /// Pre-allocated disk cache size in bytes
    pub cache_disk_size: usize,
    /// Cache disk path
    pub cache_disk_path: String,
    /// Maximum size of items to store in memory (in bytes)
    pub cache_memory_max_item_size: usize,
    /// Optional Cache-Control header value to set on responses (e.g., "public, max-age=31536000")
    pub cache_control_header: String,

    // Formats
    /// Optional list of allowed output formats (e.g., ["jpeg", "png", "avif", "webp", "jxl"]); if None, all formats are allowed
    pub allowed_output_formats: Option<Vec<String>>,
}

impl EncodingConfig {
    pub fn from_env() -> Self {
        Self {
            png_compression_level: std::env::var("IMAGE_PROXY_PNG_COMPRESSION_LEVEL")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(6),
            jpeg_quality: std::env::var("IMAGE_PROXY_JPEG_QUALITY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(75),
            avif_speed: std::env::var("IMAGE_PROXY_AVIF_SPEED")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(6),
            avif_quality: std::env::var("IMAGE_PROXY_AVIF_QUALITY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(85),
            webp_quality: std::env::var("IMAGE_PROXY_WEBP_QUALITY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(80),
            webp_effort: std::env::var("IMAGE_PROXY_WEBP_EFFORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(4),
            jxl_speed: std::env::var("IMAGE_PROXY_JXL_SPEED")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(7),
            jxl_quality: std::env::var("IMAGE_PROXY_JXL_QUALITY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(75),
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
            enable_cache: std::env::var("IMAGE_PROXY_ENABLE_CACHE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(false),
            cache_memory_size: std::env::var("IMAGE_PROXY_CACHE_MEMORY_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100 * 1024 * 1024), // Default to 100 MB
            enable_disk_cache: std::env::var("IMAGE_PROXY_ENABLE_DISK_CACHE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(false),
            cache_disk_size: std::env::var("IMAGE_PROXY_CACHE_DISK_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(512 * 1024 * 1024), // Default to 512 MB
            cache_disk_path: std::env::var("IMAGE_PROXY_CACHE_DISK_PATH")
                .unwrap_or_else(|_| "./cache".to_string()),
            cache_memory_max_item_size: std::env::var("IMAGE_PROXY_CACHE_MAX_ITEM_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1 * 1024 * 1024), // Default to 1 MB
            cache_control_header: std::env::var("IMAGE_PROXY_CACHE_CONTROL_HEADER")
                .ok()
                .unwrap_or("public, max-age=31536000, no-transform".to_string()), // Sane default for caching images for 1 year with no transformations allowed by downstream caches (Fastly, Cloudflare, etc.)
            allowed_output_formats: std::env::var("IMAGE_PROXY_ALLOWED_OUTPUT_FORMATS")
                .ok()
                .map(|s| s.split(',').map(|s| s.trim().to_string()).collect()),
        }
    }
}

impl Default for EncodingConfig {
    fn default() -> Self {
        Self {
            jpeg_quality: 75,
            png_compression_level: 6,
            avif_quality: 85,
            avif_speed: 6,
            webp_quality: 80,
            webp_effort: 4,
            jxl_speed: 7,
            jxl_quality: 75,
            resize_algorithm: ResizeAlgorithm::Auto,
            root_path: "/tmp/test-images".to_string(),
            strip_path: None,
            fallback_image_url: None,
            fallback_image_max_size: 5 * 1024 * 1024,
            enable_cache: false,
            cache_memory_size: 100 * 1024 * 1024,
            enable_disk_cache: false,
            cache_disk_size: 512 * 1024 * 1024,
            cache_disk_path: "./cache".to_string(),
            cache_memory_max_item_size: 1024 * 1024,
            cache_control_header: "public, max-age=31536000, no-transform".to_string(),
            allowed_output_formats: None,
        }
    }
}
