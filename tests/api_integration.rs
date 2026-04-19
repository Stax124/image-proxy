use std::sync::Arc;

use actix_web::{App, test, web};
use image_proxy::{
    api::image::process_image_request, api::metrics::metrics_handler, config::EncodingConfig,
    metrics::setup_metrics,
};

fn test_config(root: &str) -> Arc<EncodingConfig> {
    Arc::new(EncodingConfig {
        root_path: root.to_string(),
        ..EncodingConfig::default()
    })
}

/// Write a minimal valid JPEG to a temp path.
fn write_test_jpeg(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
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

fn build_app_data(
    config: Arc<EncodingConfig>,
) -> (
    web::Data<Arc<EncodingConfig>>,
    web::Data<awc::Client>,
    web::Data<Option<foyer::HybridCache<String, Vec<u8>>>>,
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

#[actix_web::test]
async fn serves_existing_jpeg_without_query_params() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = test_config(dir.path().to_str().unwrap());
    let (cfg, client, cache, reg, pd, rc) = build_app_data(config);

    let app = test::init_service(
        App::new()
            .app_data(cfg)
            .app_data(client)
            .app_data(cache)
            .app_data(reg)
            .app_data(pd)
            .app_data(rc)
            .service(process_image_request),
    )
    .await;

    let req = test::TestRequest::get().uri("/photo.jpeg").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "image/jpeg"
    );
}

#[actix_web::test]
async fn returns_not_found_for_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    let config = test_config(dir.path().to_str().unwrap());
    let (cfg, client, cache, reg, pd, rc) = build_app_data(config);

    let app = test::init_service(
        App::new()
            .app_data(cfg)
            .app_data(client)
            .app_data(cache)
            .app_data(reg)
            .app_data(pd)
            .app_data(rc)
            .service(process_image_request),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/nonexistent.jpeg")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);
}

#[actix_web::test]
async fn rejects_unsupported_format() {
    let dir = tempfile::tempdir().unwrap();
    let config = test_config(dir.path().to_str().unwrap());
    let (cfg, client, cache, reg, pd, rc) = build_app_data(config);

    let app = test::init_service(
        App::new()
            .app_data(cfg)
            .app_data(client)
            .app_data(cache)
            .app_data(reg)
            .app_data(pd)
            .app_data(rc)
            .service(process_image_request),
    )
    .await;

    let req = test::TestRequest::get().uri("/photo.bmp").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 415);
}

#[actix_web::test]
async fn rejects_missing_extension() {
    let dir = tempfile::tempdir().unwrap();
    let config = test_config(dir.path().to_str().unwrap());
    let (cfg, client, cache, reg, pd, rc) = build_app_data(config);

    let app = test::init_service(
        App::new()
            .app_data(cfg)
            .app_data(client)
            .app_data(cache)
            .app_data(reg)
            .app_data(pd)
            .app_data(rc)
            .service(process_image_request),
    )
    .await;

    let req = test::TestRequest::get().uri("/noextension").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 415);
}

#[actix_web::test]
async fn directory_traversal_prevented() {
    let dir = tempfile::tempdir().unwrap();
    // Create a file outside the root
    let secret = dir.path().join("secret.jpeg");
    std::fs::write(&secret, b"secret").unwrap();

    // Root is a subdir
    let root = dir.path().join("images");
    std::fs::create_dir_all(&root).unwrap();
    let config = test_config(root.to_str().unwrap());
    let (cfg, client, cache, reg, pd, rc) = build_app_data(config);

    let app = test::init_service(
        App::new()
            .app_data(cfg)
            .app_data(client)
            .app_data(cache)
            .app_data(reg)
            .app_data(pd)
            .app_data(rc)
            .service(process_image_request),
    )
    .await;

    // Attempt directory traversal
    let req = test::TestRequest::get().uri("/../secret.jpeg").to_request();
    let resp = test::call_service(&app, req).await;
    // Should NOT serve the file outside root - either 404 or the path is sanitized
    assert_eq!(resp.status(), 404);
}

#[actix_web::test]
async fn convert_format_via_query_param() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = test_config(dir.path().to_str().unwrap());
    let (cfg, client, cache, reg, pd, rc) = build_app_data(config);

    let app = test::init_service(
        App::new()
            .app_data(cfg)
            .app_data(client)
            .app_data(cache)
            .app_data(reg)
            .app_data(pd)
            .app_data(rc)
            .service(process_image_request),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/photo.jpeg?format=png")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "image/png"
    );
}

#[actix_web::test]
async fn resize_via_query_param() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = test_config(dir.path().to_str().unwrap());
    let (cfg, client, cache, reg, pd, rc) = build_app_data(config);

    let app = test::init_service(
        App::new()
            .app_data(cfg)
            .app_data(client)
            .app_data(cache)
            .app_data(reg)
            .app_data(pd)
            .app_data(rc)
            .service(process_image_request),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/photo.jpeg?size=4")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

#[actix_web::test]
async fn strip_path_works() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = Arc::new(EncodingConfig {
        root_path: dir.path().to_str().unwrap().to_string(),
        strip_path: Some("prefix/".to_string()),
        ..EncodingConfig::default()
    });
    let (cfg, client, cache, reg, pd, rc) = build_app_data(config);

    let app = test::init_service(
        App::new()
            .app_data(cfg)
            .app_data(client)
            .app_data(cache)
            .app_data(reg)
            .app_data(pd)
            .app_data(rc)
            .service(process_image_request),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/prefix/photo.jpeg")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

#[actix_web::test]
async fn vary_header_is_set() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = test_config(dir.path().to_str().unwrap());
    let (cfg, client, cache, reg, pd, rc) = build_app_data(config);

    let app = test::init_service(
        App::new()
            .app_data(cfg)
            .app_data(client)
            .app_data(cache)
            .app_data(reg)
            .app_data(pd)
            .app_data(rc)
            .service(process_image_request),
    )
    .await;

    let req = test::TestRequest::get().uri("/photo.jpeg").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("Vary").unwrap().to_str().unwrap(),
        "Sec-CH-DPR"
    );
}

#[actix_web::test]
async fn cache_control_header_default() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = test_config(dir.path().to_str().unwrap());
    let (cfg, client, cache, reg, pd, rc) = build_app_data(config);

    let app = test::init_service(
        App::new()
            .app_data(cfg)
            .app_data(client)
            .app_data(cache)
            .app_data(reg)
            .app_data(pd)
            .app_data(rc)
            .service(process_image_request),
    )
    .await;

    let req = test::TestRequest::get().uri("/photo.jpeg").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()
            .get("Cache-Control")
            .unwrap()
            .to_str()
            .unwrap(),
        "public, max-age=31536000, no-transform"
    );
}

#[actix_web::test]
async fn cache_control_header_custom() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = Arc::new(EncodingConfig {
        root_path: dir.path().to_str().unwrap().to_string(),
        cache_control_header: "public, max-age=3600".to_string(),
        ..EncodingConfig::default()
    });
    let (cfg, client, cache, reg, pd, rc) = build_app_data(config);

    let app = test::init_service(
        App::new()
            .app_data(cfg)
            .app_data(client)
            .app_data(cache)
            .app_data(reg)
            .app_data(pd)
            .app_data(rc)
            .service(process_image_request),
    )
    .await;

    let req = test::TestRequest::get().uri("/photo.jpeg").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()
            .get("Cache-Control")
            .unwrap()
            .to_str()
            .unwrap(),
        "public, max-age=3600"
    );
}

#[actix_web::test]
async fn cache_control_header_empty() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = Arc::new(EncodingConfig {
        root_path: dir.path().to_str().unwrap().to_string(),
        cache_control_header: "".to_string(),
        ..EncodingConfig::default()
    });
    let (cfg, client, cache, reg, pd, rc) = build_app_data(config);

    let app = test::init_service(
        App::new()
            .app_data(cfg)
            .app_data(client)
            .app_data(cache)
            .app_data(reg)
            .app_data(pd)
            .app_data(rc)
            .service(process_image_request),
    )
    .await;

    let req = test::TestRequest::get().uri("/photo.jpeg").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    assert!(resp.headers().get("Cache-Control").is_none());
}

#[actix_web::test]
async fn cache_control_header_with_transformations() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = Arc::new(EncodingConfig {
        root_path: dir.path().to_str().unwrap().to_string(),
        cache_control_header: "public, max-age=86400".to_string(),
        ..EncodingConfig::default()
    });
    let (cfg, client, cache, reg, pd, rc) = build_app_data(config);

    let app = test::init_service(
        App::new()
            .app_data(cfg)
            .app_data(client)
            .app_data(cache)
            .app_data(reg)
            .app_data(pd)
            .app_data(rc)
            .service(process_image_request),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/photo.jpeg?size=4&format=png")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()
            .get("Cache-Control")
            .unwrap()
            .to_str()
            .unwrap(),
        "public, max-age=86400"
    );
}

#[actix_web::test]
async fn metrics_endpoint_returns_200() {
    let (registry, _pd, _rc) = setup_metrics();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(registry))
            .service(metrics_handler),
    )
    .await;

    let req = test::TestRequest::get().uri("/metrics").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    assert!(
        resp.headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("text/plain")
    );
}

#[actix_web::test]
async fn convert_to_jxl_via_query_param() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = test_config(dir.path().to_str().unwrap());
    let (cfg, client, cache, reg, pd, rc) = build_app_data(config);

    let app = test::init_service(
        App::new()
            .app_data(cfg)
            .app_data(client)
            .app_data(cache)
            .app_data(reg)
            .app_data(pd)
            .app_data(rc)
            .service(process_image_request),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/photo.jpeg?format=jxl")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "image/jxl"
    );
}
