mod common;

use std::sync::Arc;

use actix_web::{App, test};
use common::{build_app_data, write_test_jpeg};
use image_proxy::{api::image::process_image_request, config::EncodingConfig};

#[actix_web::test]
async fn allowed_output_formats_rejects_disallowed() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = Arc::new(EncodingConfig {
        root_path: dir.path().to_str().unwrap().to_string(),
        allowed_output_formats: Some(vec!["jpeg".to_string(), "png".to_string()]),
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
        .uri("/photo.jpeg?format=webp")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 415);
}

#[actix_web::test]
async fn allowed_output_formats_permits_allowed() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = Arc::new(EncodingConfig {
        root_path: dir.path().to_str().unwrap().to_string(),
        allowed_output_formats: Some(vec!["jpeg".to_string(), "png".to_string()]),
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
async fn allowed_output_formats_none_allows_all() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = Arc::new(EncodingConfig {
        root_path: dir.path().to_str().unwrap().to_string(),
        allowed_output_formats: None,
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
        .uri("/photo.jpeg?format=webp")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

#[actix_web::test]
async fn allowed_output_formats_no_format_param_bypasses_check() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    // Only allow png, but request without format param should still work
    let config = Arc::new(EncodingConfig {
        root_path: dir.path().to_str().unwrap().to_string(),
        allowed_output_formats: Some(vec!["png".to_string()]),
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
}
