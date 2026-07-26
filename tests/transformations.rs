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

    for alg in ["thumbnail", "lanczos3", "bicubic", "auto"] {
        let req = test::TestRequest::get()
            .uri(&format!("/photo.jpeg?size=4&resize_algorithm={}", alg))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200, "resize_algorithm={} failed", alg);
    }
}

// --- Black-and-white (bw) ---

#[actix_web::test]
async fn bw_converts_color_jpeg_to_grayscale() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = test_config(dir.path().to_str().unwrap());
    let app = init_test_app!(config);

    let req = test::TestRequest::get()
        .uri("/photo.jpeg?bw=1&format=png")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body = test::read_body(resp).await;
    let decoded = image::load_from_memory(&body).expect("decode bw result");
    // True Luma or RGB with R=G=B after round-trip through PNG
    let rgb = decoded.to_rgb8();
    for pixel in rgb.pixels() {
        assert_eq!(pixel[0], pixel[1], "R must equal G for grayscale");
        assert_eq!(pixel[1], pixel[2], "G must equal B for grayscale");
    }
}

#[actix_web::test]
async fn bw_preserves_alpha_on_png() {
    let dir = tempfile::tempdir().unwrap();
    write_test_png_with_alpha(dir.path(), "photo.png");
    let config = test_config(dir.path().to_str().unwrap());
    let app = init_test_app!(config);

    let req = test::TestRequest::get()
        .uri("/photo.png?bw=1&format=png")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body = test::read_body(resp).await;
    let decoded = image::load_from_memory(&body).expect("decode bw+alpha result");
    assert!(
        decoded.color().has_alpha(),
        "alpha channel should be preserved under bw"
    );
    let rgba = decoded.to_rgba8();
    // Source alpha varies with (x*16 + y*16); at least one non-opaque pixel should remain
    let has_partial_alpha = rgba.pixels().iter().any(|p| p[3] > 0 && p[3] < 255);
    assert!(
        has_partial_alpha,
        "expected partial alpha to survive bw conversion"
    );
    for pixel in rgba.pixels() {
        assert_eq!(pixel[0], pixel[1], "R must equal G for grayscale");
        assert_eq!(pixel[1], pixel[2], "G must equal B for grayscale");
    }
}

#[actix_web::test]
async fn bw_combines_with_size_and_format() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("photo.jpeg"), common::make_test_jpeg_bytes(64, 64)).unwrap();
    let config = test_config(dir.path().to_str().unwrap());
    let app = init_test_app!(config);

    let req = test::TestRequest::get()
        .uri("/photo.jpeg?bw=1&size=16&format=webp")
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

    let body = test::read_body(resp).await;
    let decoded = image::load_from_memory(&body).expect("decode");
    assert_eq!(decoded.width(), 16);
    assert_eq!(decoded.height(), 16);
    // Lossy WebP can drift channels by a few units after Luma→RGB expand; allow a small delta
    let rgb = decoded.to_rgb8();
    for pixel in rgb.pixels() {
        assert!(
            (pixel[0] as i16 - pixel[1] as i16).abs() <= 3
                && (pixel[1] as i16 - pixel[2] as i16).abs() <= 3,
            "expected near-grayscale after bw+webp, got {:?}",
            pixel
        );
    }
}

#[actix_web::test]
async fn bw_falsey_values_leave_color() {
    let dir = tempfile::tempdir().unwrap();
    // Highly saturated solid red so color vs gray is unambiguous after JPEG round-trip
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

    let config = test_config(dir.path().to_str().unwrap());
    let app = init_test_app!(config);

    let req = test::TestRequest::get()
        .uri("/red.png?bw=0&format=png")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body = test::read_body(resp).await;
    let decoded = image::load_from_memory(&body).expect("decode");
    let rgb = decoded.to_rgb8();
    let pixel = rgb.get_pixel(0, 0);
    // Still red-ish, not equal gray channels
    assert!(
        pixel[0] > pixel[1] && pixel[0] > pixel[2],
        "bw=0 should not desaturate; got {:?}",
        pixel
    );
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
