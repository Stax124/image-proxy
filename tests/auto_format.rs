#[macro_use]
mod common;

use std::sync::Arc;

use actix_web::test;
use common::write_test_jpeg;
use image_proxy::config::EncodingConfig;

// ── Auto-format via Accept header (preferred_formats) ───────────────

#[actix_web::test]
async fn auto_format_returns_webp_when_accept_supports_it() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = Arc::new(EncodingConfig {
        root_path: dir.path().to_str().unwrap().to_string(),
        preferred_formats: Some(vec!["webp".to_string()]),
        ..EncodingConfig::default()
    });
    let app = init_test_app!(config);

    // A size param triggers the pipeline; Accept header signals webp support
    let req = test::TestRequest::get()
        .uri("/photo.jpeg")
        .insert_header(("Accept", "image/webp,image/jpeg,*/*"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "image/webp"
    );
}

#[actix_web::test]
async fn auto_format_returns_avif_when_preferred_and_accepted() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = Arc::new(EncodingConfig {
        root_path: dir.path().to_str().unwrap().to_string(),
        preferred_formats: Some(vec!["avif".to_string(), "webp".to_string()]),
        ..EncodingConfig::default()
    });
    let app = init_test_app!(config);

    // Chrome-like Accept header that supports avif
    let req = test::TestRequest::get()
        .uri("/photo.jpeg")
        .insert_header(("Accept", "image/avif,image/webp,image/apng,*/*;q=0.8"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "image/avif"
    );
}

#[actix_web::test]
async fn auto_format_falls_back_when_browser_does_not_support_preferred() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = Arc::new(EncodingConfig {
        root_path: dir.path().to_str().unwrap().to_string(),
        preferred_formats: Some(vec!["jxl".to_string(), "avif".to_string()]),
        ..EncodingConfig::default()
    });
    let app = init_test_app!(config);

    // Browser only supports jpeg and png — none of the preferred formats
    let req = test::TestRequest::get()
        .uri("/photo.jpeg")
        .insert_header(("Accept", "image/jpeg,image/png"))
        .to_request();
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
async fn auto_format_explicit_format_param_overrides_accept_header() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = Arc::new(EncodingConfig {
        root_path: dir.path().to_str().unwrap().to_string(),
        preferred_formats: Some(vec!["webp".to_string()]),
        ..EncodingConfig::default()
    });
    let app = init_test_app!(config);

    // Explicit format=png should override the Accept-based auto-format
    let req = test::TestRequest::get()
        .uri("/photo.jpeg?format=png")
        .insert_header(("Accept", "image/webp,image/jpeg,*/*"))
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
async fn auto_format_disabled_when_preferred_formats_not_configured() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = Arc::new(EncodingConfig {
        root_path: dir.path().to_str().unwrap().to_string(),
        preferred_formats: None,
        ..EncodingConfig::default()
    });
    let app = init_test_app!(config);

    // Browser supports webp, but preferred_formats is None — should keep original jpeg
    let req = test::TestRequest::get()
        .uri("/photo.jpeg")
        .insert_header(("Accept", "image/webp,image/jpeg,*/*"))
        .to_request();
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
async fn auto_format_respects_allowed_output_formats() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = Arc::new(EncodingConfig {
        root_path: dir.path().to_str().unwrap().to_string(),
        preferred_formats: Some(vec!["avif".to_string(), "webp".to_string()]),
        allowed_output_formats: Some(vec!["jpeg".to_string(), "webp".to_string()]),
        ..EncodingConfig::default()
    });
    let app = init_test_app!(config);

    // Browser supports both avif and webp, but allowed_output_formats excludes avif
    let req = test::TestRequest::get()
        .uri("/photo.jpeg")
        .insert_header(("Accept", "image/avif,image/webp,image/jpeg,*/*"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    // avif is preferred but not allowed, so should fall back to webp
    assert_eq!(
        resp.headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "image/webp"
    );
}
