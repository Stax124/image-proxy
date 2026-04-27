use std::sync::Arc;

use actix_web::web;
use image_proxy::{config::EncodingConfig, metrics::setup_metrics};

#[allow(dead_code)]
pub fn test_config(root: &str) -> Arc<EncodingConfig> {
    Arc::new(EncodingConfig {
        root_path: root.to_string(),
        ..EncodingConfig::default()
    })
}

#[allow(dead_code)]
/// Write a minimal valid JPEG to a temp path.
pub fn write_test_jpeg(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    let rgba = image::RgbaImage::from_fn(8, 8, |x, y| {
        image::Rgba([(x * 32) as u8, (y * 32) as u8, 128, 255])
    });
    let rgb = image::DynamicImage::ImageRgba8(rgba).into_rgb8();
    let mut buf = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 75);
    image::ImageEncoder::write_image(encoder, rgb.as_raw(), 8, 8, image::ExtendedColorType::Rgb8)
        .unwrap();
    std::fs::write(&path, &buf).unwrap();
    path
}

pub fn build_app_data(
    config: Arc<EncodingConfig>,
) -> (
    web::Data<Arc<EncodingConfig>>,
    web::Data<awc::Client>,
    web::Data<Option<foyer::HybridCache<String, bytes::Bytes>>>,
    web::Data<prometheus::Registry>,
    web::Data<prometheus::HistogramVec>,
    web::Data<prometheus::IntCounterVec>,
) {
    let (registry, pipeline_duration, request_count) = setup_metrics();
    let http_client = awc::Client::default();
    (
        web::Data::new(config),
        web::Data::new(http_client),
        web::Data::new(None),
        web::Data::new(registry),
        web::Data::new(pipeline_duration),
        web::Data::new(request_count),
    )
}
