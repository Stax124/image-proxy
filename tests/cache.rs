#[macro_use]
mod common;

use std::sync::Arc;

use actix_web::{App, test};
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
    std::fs::write(dir.path().join("photo.jpeg"), make_test_jpeg_bytes(32, 32)).unwrap();

    let (cfg, client, cache, reg, pd, rc) = make_cached_data(dir.path().to_str().unwrap()).await;
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
    std::fs::write(dir.path().join("photo.jpeg"), make_test_jpeg_bytes(32, 32)).unwrap();

    let (cfg, client, cache, reg, pd, rc) = make_cached_data(dir.path().to_str().unwrap()).await;
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

/// `bw` must participate in the cache key so color and grayscale variants
/// never collide.
#[actix_web::test]
async fn cache_bw_param_produces_separate_entries() {
    let dir = tempfile::tempdir().unwrap();
    // Saturated red so color vs gray is unambiguous after re-encode
    let rgba = image::RgbaImage::from_pixel(16, 16, image::Rgba([255, 0, 0, 255]));
    let rgb = image::DynamicImage::ImageRgba8(rgba).into_rgb8();
    let mut buf = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    image::ImageEncoder::write_image(
        encoder,
        rgb.as_raw(),
        16,
        16,
        image::ExtendedColorType::Rgb8,
    )
    .unwrap();
    std::fs::write(dir.path().join("red.png"), &buf).unwrap();

    let (cfg, client, cache, reg, pd, rc) = make_cached_data(dir.path().to_str().unwrap()).await;
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

    // Color variant (format forces a transform so the response is cached)
    let color_uri = "/red.png?format=png";
    let req_color = test::TestRequest::get().uri(color_uri).to_request();
    let resp_color = test::call_service(&app, req_color).await;
    assert_eq!(resp_color.status(), 200);
    assert_eq!(
        resp_color
            .headers()
            .get("X-Image-Proxy-Cache")
            .unwrap()
            .to_str()
            .unwrap(),
        "MISS"
    );
    let body_color = test::read_body(resp_color).await;
    let color_img = image::load_from_memory(&body_color).expect("decode color");
    let color_rgb = color_img.to_rgb8();
    // Source is solid red — at least some pixels should remain non-gray
    let has_color = color_rgb
        .pixels()
        .iter()
        .any(|p| p[0] != p[1] || p[1] != p[2]);
    assert!(has_color, "non-bw response should retain color");

    // Grayscale variant must be a separate key (MISS, not color HIT)
    let bw_uri = "/red.png?format=png&bw=1";
    let req_bw = test::TestRequest::get().uri(bw_uri).to_request();
    let resp_bw = test::call_service(&app, req_bw).await;
    assert_eq!(resp_bw.status(), 200);
    assert_eq!(
        resp_bw
            .headers()
            .get("X-Image-Proxy-Cache")
            .unwrap()
            .to_str()
            .unwrap(),
        "MISS",
        "bw=1 must not share the color cache entry"
    );
    let body_bw = test::read_body(resp_bw).await;
    let bw_img = image::load_from_memory(&body_bw).expect("decode bw");
    let bw_rgb = bw_img.to_rgb8();
    for pixel in bw_rgb.pixels() {
        assert_eq!(pixel[0], pixel[1], "R must equal G for grayscale");
        assert_eq!(pixel[1], pixel[2], "G must equal B for grayscale");
    }
    assert_ne!(
        body_color, body_bw,
        "color and bw responses must differ in body"
    );

    // Repeat each URI -> HIT on its own entry
    let resp_color2 =
        test::call_service(&app, test::TestRequest::get().uri(color_uri).to_request()).await;
    assert_eq!(
        resp_color2
            .headers()
            .get("X-Image-Proxy-Cache")
            .unwrap()
            .to_str()
            .unwrap(),
        "HIT"
    );
    assert_eq!(test::read_body(resp_color2).await, body_color);

    let resp_bw2 =
        test::call_service(&app, test::TestRequest::get().uri(bw_uri).to_request()).await;
    assert_eq!(
        resp_bw2
            .headers()
            .get("X-Image-Proxy-Cache")
            .unwrap()
            .to_str()
            .unwrap(),
        "HIT"
    );
    assert_eq!(test::read_body(resp_bw2).await, body_bw);
}
