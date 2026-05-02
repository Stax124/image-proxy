#[macro_use]
mod common;

use std::sync::Arc;

use actix_web::test;
use common::{test_config, write_test_jpeg, write_test_png_with_alpha};
use image_proxy::config::EncodingConfig;

#[actix_web::test]
async fn convert_format_via_query_param() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = test_config(dir.path().to_str().unwrap());
    let app = init_test_app!(config);

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
    let app = init_test_app!(config);

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
    let app = init_test_app!(config);

    let req = test::TestRequest::get()
        .uri("/prefix/photo.jpeg")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

#[actix_web::test]
async fn convert_to_jxl_via_query_param() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = test_config(dir.path().to_str().unwrap());
    let app = init_test_app!(config);

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

#[actix_web::test]
async fn convert_to_webp_from_opaque_image() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = test_config(dir.path().to_str().unwrap());
    let app = init_test_app!(config);

    let req = test::TestRequest::get()
        .uri("/photo.jpeg?format=webp")
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
async fn convert_to_webp_from_alpha_image() {
    let dir = tempfile::tempdir().unwrap();
    write_test_png_with_alpha(dir.path(), "photo.png");
    let config = test_config(dir.path().to_str().unwrap());
    let app = init_test_app!(config);

    let req = test::TestRequest::get()
        .uri("/photo.png?format=webp")
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
async fn convert_to_jxl_from_alpha_image() {
    let dir = tempfile::tempdir().unwrap();
    write_test_png_with_alpha(dir.path(), "photo.png");
    let config = test_config(dir.path().to_str().unwrap());
    let app = init_test_app!(config);

    let req = test::TestRequest::get()
        .uri("/photo.png?format=jxl")
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
