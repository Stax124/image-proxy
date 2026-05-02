#[macro_use]
mod common;

use std::sync::Arc;

use actix_web::test;
use common::{test_config, write_test_jpeg};
use image_proxy::config::EncodingConfig;

#[actix_web::test]
async fn vary_header_is_set() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = test_config(dir.path().to_str().unwrap());
    let app = init_test_app!(config);

    let req = test::TestRequest::get().uri("/photo.jpeg").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("Vary").unwrap().to_str().unwrap(),
        "Accept, Sec-CH-DPR"
    );
}

#[actix_web::test]
async fn cache_control_header_default() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = test_config(dir.path().to_str().unwrap());
    let app = init_test_app!(config);

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
    let app = init_test_app!(config);

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
    let app = init_test_app!(config);

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
    let app = init_test_app!(config);

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
async fn cache_status_header_default() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = test_config(dir.path().to_str().unwrap());
    let app = init_test_app!(config);

    let req = test::TestRequest::get()
        .uri("/photo.jpeg?format=jpeg")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()
            .get("X-Image-Proxy-Cache")
            .unwrap()
            .to_str()
            .unwrap(),
        "MISS"
    );
}

#[actix_web::test]
async fn cache_status_header_custom_name() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = Arc::new(EncodingConfig {
        root_path: dir.path().to_str().unwrap().to_string(),
        cache_status_header: "X-My-Cache".to_string(),
        ..EncodingConfig::default()
    });
    let app = init_test_app!(config);

    let req = test::TestRequest::get()
        .uri("/photo.jpeg?format=jpeg")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("X-My-Cache").unwrap().to_str().unwrap(),
        "MISS"
    );
    // Default header should not be present
    assert!(resp.headers().get("X-Image-Proxy-Cache").is_none());
}
