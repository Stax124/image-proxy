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

// --- DPR and resize_algorithm primary logic ---

#[actix_web::test]
async fn dpr_query_param_multiplies_size() {
    let dir = tempfile::tempdir().unwrap();
    // Larger source so that effective size after DPR is observable (not clamped to source)
    std::fs::write(dir.path().join("photo.jpeg"), common::make_test_jpeg_bytes(64, 64)).unwrap();
    let config = test_config(dir.path().to_str().unwrap());
    let app = init_test_app!(config);

    // size=16 * dpr=2 => effective 32
    let req = test::TestRequest::get()
        .uri("/photo.jpeg?size=16&dpr=2")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body = test::read_body(resp).await;
    let decoded = image::load_from_memory(&body).expect("decode dpr result");
    // Square source -> long edge should be the effective size (32), clamped logic already applied inside
    assert_eq!(decoded.width(), 32);
    assert_eq!(decoded.height(), 32);
}

#[actix_web::test]
async fn dpr_from_sec_ch_dpr_header() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("photo.jpeg"), common::make_test_jpeg_bytes(64, 64)).unwrap();
    let config = test_config(dir.path().to_str().unwrap());
    let app = init_test_app!(config);

    let req = test::TestRequest::get()
        .uri("/photo.jpeg?size=16")
        .insert_header(("Sec-CH-DPR", "2"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body = test::read_body(resp).await;
    let decoded = image::load_from_memory(&body).expect("decode sec-ch-dpr result");
    assert_eq!(decoded.width(), 32);
}

#[actix_web::test]
async fn dpr_out_of_range_is_ignored() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("photo.jpeg"), common::make_test_jpeg_bytes(64, 64)).unwrap();
    let config = test_config(dir.path().to_str().unwrap());
    let app = init_test_app!(config);

    // dpr=0.5 is below 1.0 -> ignored, effective size stays 16
    let req = test::TestRequest::get()
        .uri("/photo.jpeg?size=16&dpr=0.5")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body = test::read_body(resp).await;
    let decoded = image::load_from_memory(&body).expect("decode");
    assert_eq!(decoded.width(), 16);
}

#[actix_web::test]
async fn resize_algorithm_param_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = test_config(dir.path().to_str().unwrap());
    let app = init_test_app!(config);

    for alg in ["thumbnail", "lanczos3", "auto"] {
        let req = test::TestRequest::get()
            .uri(&format!("/photo.jpeg?size=4&resize_algorithm={}", alg))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200, "resize_algorithm={} failed", alg);
    }
}

// --- Pipeline error path for corrupt input ---

#[actix_web::test]
async fn corrupt_image_data_returns_500_when_pipeline_required() {
    let dir = tempfile::tempdir().unwrap();
    let bad_path = dir.path().join("bad.jpeg");
    // Extension is supported but content is not a valid image -> decode inside pipeline must fail
    std::fs::write(&bad_path, b"\xff\xd8\xff not a real jpeg payload at all").unwrap();

    let config = test_config(dir.path().to_str().unwrap());
    let app = init_test_app!(config);

    // Without a transform query the direct-stream path would succeed (raw bytes).
    // Force the pipeline by requesting a conversion.
    let req = test::TestRequest::get()
        .uri("/bad.jpeg?format=png")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 500);
}
