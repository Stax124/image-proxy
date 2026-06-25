use std::sync::Arc;

use actix_web::web;
use image_proxy::{config::EncodingConfig, metrics::setup_metrics};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;

#[allow(unused_macros)]
macro_rules! init_test_app {
    ($config:expr) => {{
        let (cfg, client, cache, reg, pd, rc) = common::build_app_data($config);
        ::actix_web::test::init_service(
            ::actix_web::App::new()
                .app_data(cfg)
                .app_data(client)
                .app_data(cache)
                .app_data(reg)
                .app_data(pd)
                .app_data(rc)
                .service(::image_proxy::api::image::process_image_request),
        )
        .await
    }};
}

#[allow(dead_code)]
pub fn test_config(root: &str) -> Arc<EncodingConfig> {
    Arc::new(EncodingConfig {
        root_path: root.to_string(),
        ..EncodingConfig::default()
    })
}

/// Generate JPEG bytes of the given size (no filesystem side effects).
#[allow(dead_code)]
pub fn make_test_jpeg_bytes(w: u32, h: u32) -> Vec<u8> {
    let rgba = image::RgbaImage::from_fn(w, h, |x, y| {
        image::Rgba([(x * 32) as u8, (y * 32) as u8, 128, 255])
    });
    let rgb = image::DynamicImage::ImageRgba8(rgba).into_rgb8();
    let mut buf = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 75);
    image::ImageEncoder::write_image(encoder, rgb.as_raw(), w, h, image::ExtendedColorType::Rgb8)
        .unwrap();
    buf
}

/// Generate PNG (with alpha) bytes of the given size.
#[allow(dead_code)]
pub fn make_test_png_with_alpha_bytes(w: u32, h: u32) -> Vec<u8> {
    let rgba = image::RgbaImage::from_fn(w, h, |x, y| {
        image::Rgba([(x * 32) as u8, (y * 32) as u8, 128, (x * 16 + y * 16) as u8])
    });
    let mut buf = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    image::ImageEncoder::write_image(
        encoder,
        rgba.as_raw(),
        w,
        h,
        image::ExtendedColorType::Rgba8,
    )
    .unwrap();
    buf
}

#[allow(dead_code)]
/// Write a minimal valid JPEG to a temp path.
pub fn write_test_jpeg(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    let buf = make_test_jpeg_bytes(8, 8);
    std::fs::write(&path, &buf).unwrap();
    path
}

#[allow(dead_code)]
/// Write a minimal valid PNG with alpha channel to a temp path.
pub fn write_test_png_with_alpha(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    let buf = make_test_png_with_alpha_bytes(8, 8);
    std::fs::write(&path, &buf).unwrap();
    path
}

pub type AppData = (
    web::Data<Arc<EncodingConfig>>,
    web::Data<awc::Client>,
    web::Data<Option<foyer::HybridCache<String, bytes::Bytes>>>,
    web::Data<prometheus::Registry>,
    web::Data<prometheus::HistogramVec>,
    web::Data<prometheus::IntCounterVec>,
);

#[allow(dead_code)]
pub fn build_app_data(config: Arc<EncodingConfig>) -> AppData {
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

/// Build AppData with a real in-memory foyer cache enabled.
/// Uses the provided config (caller should set enable_cache: true).
#[allow(dead_code)]
pub async fn build_app_data_with_cache(config: Arc<EncodingConfig>) -> AppData {
    let (registry, pipeline_duration, request_count) = setup_metrics();
    let http_client = awc::Client::default();
    let hybrid_cache = image_proxy::cache::setup_cache(&config, &registry)
        .await
        .expect("failed to setup cache for test");
    (
        web::Data::new(config),
        web::Data::new(http_client),
        web::Data::new(hybrid_cache),
        web::Data::new(registry),
        web::Data::new(pipeline_duration),
        web::Data::new(request_count),
    )
}

/// Start a minimal HTTP/1.1 server on a random localhost port that responds to every GET
/// with the supplied bytes and Content-Type. Intended only for fallback_image_url tests.
/// Returns (base_url_with_trailing_slash, join_handle). The task runs until the test ends.
#[allow(dead_code)]
pub async fn start_simple_http_server(
    data: Vec<u8>,
    content_type: &'static str,
) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.expect("bind temp http server");
    let addr = listener.local_addr().expect("local addr");
    let base = format!("http://{}/", addr);

    let handle = tokio::spawn(async move {
        loop {
            let (mut socket, _) = match listener.accept().await {
                Ok(v) => v,
                Err(_) => break,
            };

            // Read (and ignore) the request headers minimally.
            let mut buf = [0u8; 1024];
            let _ = socket.readable().await;
            let _ = socket.try_read(&mut buf);

            // Write a minimal HTTP response.
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                content_type,
                data.len()
            );
            let _ = socket.write_all(header.as_bytes()).await;
            let _ = socket.write_all(&data).await;
            let _ = socket.flush().await;
            // Drop closes the connection.
        }
    });

    (base, handle)
}
