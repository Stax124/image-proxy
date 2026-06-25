#[macro_use]
mod common;

use std::sync::Arc;

use actix_web::{test, App};
use common::{build_app_data_with_cache, make_test_jpeg_bytes};
use image_proxy::config::EncodingConfig;

/// Build a cached app config + the tuple needed to init the service.
async fn make_cached_data(root: &str) -> common::AppData {
    let mut cfg = EncodingConfig {
        root_path: root.to_string(),
        enable_cache: true,
        // Keep memory footprint tiny for tests
        cache_memory_size: 4 * 1024 * 1024,
        cache_memory_max_item_size: 2 * 1024 * 1024,
        ..EncodingConfig::default()
    };
    cfg.cache_status_header = "X-Image-Proxy-Cache".to_string();
    build_app_data_with_cache(Arc::new(cfg)).await
}

#[actix_web::test]
async fn cache_miss_then_hit_for_transformed_request() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("photo.jpeg"),
        make_test_jpeg_bytes(32, 32),
    )
    .unwrap();

    let (cfg, client, cache, reg, pd, rc) =
        make_cached_data(dir.path().to_str().unwrap()).await;
    let app = test::init_service(
        App::new()
            .app_data(cfg)
            .app_data(client)
            .app_data(cache)
            .app_data(reg)
            .app_data(pd)
            .app_data(rc)
            .service(image_proxy::api::image::process_image_request),
    )
    .await;

    let uri = "/photo.jpeg?format=webp&size=16";

    // First request -> MISS (processed and inserted)
    let req1 = test::TestRequest::get().uri(uri).to_request();
    let resp1 = test::call_service(&app, req1).await;
    assert_eq!(resp1.status(), 200);
    let header1 = resp1
        .headers()
        .get("X-Image-Proxy-Cache")
        .map(|v| v.to_str().unwrap_or(""))
        .unwrap_or("");
    assert_eq!(header1, "MISS");

    let body1 = test::read_body(resp1).await;

    // Second identical request -> HIT (served from cache, no re-process)
    let req2 = test::TestRequest::get().uri(uri).to_request();
    let resp2 = test::call_service(&app, req2).await;
    assert_eq!(resp2.status(), 200);
    let header2 = resp2
        .headers()
        .get("X-Image-Proxy-Cache")
        .map(|v| v.to_str().unwrap_or(""))
        .unwrap_or("");
    assert_eq!(header2, "HIT");

    let body2 = test::read_body(resp2).await;
    assert_eq!(body1, body2);
}

#[actix_web::test]
async fn cache_different_params_produce_different_entries() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("photo.jpeg"),
        make_test_jpeg_bytes(32, 32),
    )
    .unwrap();

    let (cfg, client, cache, reg, pd, rc) =
        make_cached_data(dir.path().to_str().unwrap()).await;
    let app = test::init_service(
        App::new()
            .app_data(cfg)
            .app_data(client)
            .app_data(cache)
            .app_data(reg)
            .app_data(pd)
            .app_data(rc)
            .service(image_proxy::api::image::process_image_request),
    )
    .await;

    // Request A
    let r1 = test::TestRequest::get()
        .uri("/photo.jpeg?format=webp&size=8")
        .to_request();
    let resp1 = test::call_service(&app, r1).await;
    assert_eq!(resp1.status(), 200);
    assert_eq!(
        resp1
            .headers()
            .get("X-Image-Proxy-Cache")
            .unwrap()
            .to_str()
            .unwrap(),
        "MISS"
    );

    // Request B with different size (different cache key) -> also MISS
    let r2 = test::TestRequest::get()
        .uri("/photo.jpeg?format=webp&size=12")
        .to_request();
    let resp2 = test::call_service(&app, r2).await;
    assert_eq!(resp2.status(), 200);
    assert_eq!(
        resp2
            .headers()
            .get("X-Image-Proxy-Cache")
            .unwrap()
            .to_str()
            .unwrap(),
        "MISS"
    );
}