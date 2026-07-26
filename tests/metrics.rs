#[macro_use]
mod common;

use std::sync::Arc;

use actix_web::{App, test, web};
use common::{
    build_app_data, build_app_data_with_cache, make_test_jpeg_bytes, test_config, write_test_jpeg,
};
use image_proxy::{
    api::metrics::metrics_handler,
    config::EncodingConfig,
    metrics::{RejectFormat, RequestPath, RequestStatus, RequestTracker, setup_metrics},
};
use prometheus::{Encoder, Registry, TextEncoder};

fn gather_text(registry: &Registry) -> String {
    let families = registry.gather();
    let encoder = TextEncoder::new();
    let mut buffer = Vec::new();
    encoder.encode(&families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

/// True if a non-comment sample line matches metric name and all label fragments.
fn has_sample(text: &str, metric: &str, labels: &[&str]) -> bool {
    text.lines().any(|line| {
        !line.starts_with('#')
            && line.contains(metric)
            && labels.iter().all(|l| line.contains(l))
    })
}

#[actix_web::test]
async fn metrics_endpoint_returns_200() {
    let (registry, _metrics) = setup_metrics();

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
async fn metrics_endpoint_exposes_core_series() {
    let (registry, metrics) = setup_metrics();

    // Simulate one finished request so request series appear with samples.
    {
        let mut tracker = RequestTracker::new(metrics.clone());
        tracker.set_format("jpeg");
        tracker.ok(RequestPath::PassThrough, 1024);
    }
    // HistogramVec only emits series after an observation.
    metrics
        .pipeline_duration
        .with_label_values(&["encode", "jpeg"])
        .observe(0.01);

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(registry))
            .service(metrics_handler),
    )
    .await;

    let req = test::TestRequest::get().uri("/metrics").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body = test::read_body(resp).await;
    let text = String::from_utf8_lossy(&body);

    assert!(
        text.contains("image_requests_total"),
        "missing image_requests_total"
    );
    assert!(
        text.contains("image_request_duration_seconds"),
        "missing image_request_duration_seconds"
    );
    assert!(
        text.contains("image_pipeline_step_duration_seconds"),
        "missing image_pipeline_step_duration_seconds"
    );
    assert!(
        text.contains("image_response_bytes_total"),
        "missing image_response_bytes_total"
    );
    assert!(
        text.contains("image_requests_in_flight"),
        "missing image_requests_in_flight"
    );
    assert!(
        text.contains("path=\"pass_through\""),
        "expected path label sample: {}",
        text
    );
}

#[actix_web::test]
async fn request_tracker_records_exactly_once() {
    let (registry, metrics) = setup_metrics();

    {
        let mut tracker = RequestTracker::new(metrics.clone());
        tracker.set_format("webp");
        tracker.ok(RequestPath::Transform, 42);
        tracker.finish();
        // Second finish must be a no-op
        tracker.finish();
    }

    let text = gather_text(&registry);

    // Counter should be 1, not 2
    let line = text
        .lines()
        .find(|l| {
            l.contains("image_requests_total")
                && l.contains("format=\"webp\"")
                && l.contains("path=\"transform\"")
                && !l.starts_with('#')
        })
        .expect("request counter sample");
    assert!(
        line.ends_with('1') || line.contains("} 1"),
        "expected count 1, got: {line}"
    );

    assert_eq!(
        metrics.in_flight.get(),
        0,
        "in_flight should return to zero after finish"
    );
}

#[actix_web::test]
async fn request_tracker_reject_and_fail_labels() {
    let (registry, metrics) = setup_metrics();

    {
        let mut t = RequestTracker::new(metrics.clone());
        t.reject(RejectFormat::Unsupported, RequestStatus::UnsupportedMediaType);
    }
    {
        let mut t = RequestTracker::new(metrics.clone());
        t.reject(RejectFormat::Unknown, RequestStatus::UnsupportedMediaType);
    }
    {
        let mut t = RequestTracker::new(metrics.clone());
        t.fail(RequestPath::NotFound, RequestStatus::NotFound);
    }

    let text = gather_text(&registry);
    assert!(
        has_sample(
            &text,
            "image_requests_total",
            &[
                "format=\"unsupported\"",
                "status=\"unsupported_media_type\"",
                "path=\"rejected\"",
            ]
        ),
        "reject unsupported sample missing: {text}"
    );
    assert!(
        has_sample(
            &text,
            "image_requests_total",
            &[
                "format=\"unknown\"",
                "status=\"unsupported_media_type\"",
                "path=\"rejected\"",
            ]
        ),
        "reject unknown sample missing: {text}"
    );
    assert!(
        has_sample(
            &text,
            "image_requests_total",
            &["status=\"not_found\"", "path=\"not_found\""]
        ),
        "fail sample missing: {text}"
    );
}

/// Integration: real handler outcomes produce the expected path/status labels.
#[actix_web::test]
async fn handler_labels_pass_through() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = test_config(dir.path().to_str().unwrap());
    let (cfg, client, cache, registry, app_metrics) = build_app_data(config);
    let app = test::init_service(
        App::new()
            .app_data(cfg)
            .app_data(client)
            .app_data(cache)
            .app_data(registry.clone())
            .app_data(app_metrics)
            .service(image_proxy::api::image::process_image_request),
    )
    .await;

    let resp = test::call_service(
        &app,
        test::TestRequest::get().uri("/photo.jpeg").to_request(),
    )
    .await;
    assert_eq!(resp.status(), 200);
    // Drain body so response completes cleanly
    let _ = test::read_body(resp).await;

    let text = gather_text(registry.get_ref());
    assert!(
        has_sample(
            &text,
            "image_requests_total",
            &[
                "status=\"ok\"",
                "path=\"pass_through\"",
                "format=\"jpeg\"",
            ]
        ),
        "pass_through labels missing: {text}"
    );
}

#[actix_web::test]
async fn handler_labels_transform() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = test_config(dir.path().to_str().unwrap());
    let (cfg, client, cache, registry, app_metrics) = build_app_data(config);
    let app = test::init_service(
        App::new()
            .app_data(cfg)
            .app_data(client)
            .app_data(cache)
            .app_data(registry.clone())
            .app_data(app_metrics)
            .service(image_proxy::api::image::process_image_request),
    )
    .await;

    let resp = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/photo.jpeg?size=8")
            .to_request(),
    )
    .await;
    assert_eq!(resp.status(), 200);
    let _ = test::read_body(resp).await;

    let text = gather_text(registry.get_ref());
    assert!(
        has_sample(
            &text,
            "image_requests_total",
            &["status=\"ok\"", "path=\"transform\""]
        ),
        "transform labels missing: {text}"
    );
}

#[actix_web::test]
async fn handler_labels_rejected() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("file.exe"), b"not an image").unwrap();
    let config = test_config(dir.path().to_str().unwrap());
    let (cfg, client, cache, registry, app_metrics) = build_app_data(config);
    let app = test::init_service(
        App::new()
            .app_data(cfg)
            .app_data(client)
            .app_data(cache)
            .app_data(registry.clone())
            .app_data(app_metrics)
            .service(image_proxy::api::image::process_image_request),
    )
    .await;

    let resp = test::call_service(
        &app,
        test::TestRequest::get().uri("/file.exe").to_request(),
    )
    .await;
    assert_eq!(resp.status(), 415);

    let text = gather_text(registry.get_ref());
    assert!(
        has_sample(
            &text,
            "image_requests_total",
            &[
                "format=\"unsupported\"",
                "status=\"unsupported_media_type\"",
                "path=\"rejected\"",
            ]
        ),
        "rejected labels missing: {text}"
    );
    // Raw extension must not become a metric label (cardinality).
    assert!(
        !text.lines().any(|l| {
            !l.starts_with('#') && l.contains("image_requests_total") && l.contains("format=\"exe\"")
        }),
        "raw extension leaked into format label: {text}"
    );
}

#[actix_web::test]
async fn handler_labels_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let config = test_config(dir.path().to_str().unwrap());
    let (cfg, client, cache, registry, app_metrics) = build_app_data(config);
    let app = test::init_service(
        App::new()
            .app_data(cfg)
            .app_data(client)
            .app_data(cache)
            .app_data(registry.clone())
            .app_data(app_metrics)
            .service(image_proxy::api::image::process_image_request),
    )
    .await;

    let resp = test::call_service(
        &app,
        test::TestRequest::get().uri("/missing.jpeg").to_request(),
    )
    .await;
    assert_eq!(resp.status(), 404);

    let text = gather_text(registry.get_ref());
    assert!(
        has_sample(
            &text,
            "image_requests_total",
            &["status=\"not_found\"", "path=\"not_found\""]
        ),
        "not_found labels missing: {text}"
    );
}

#[actix_web::test]
async fn handler_labels_non_processable() {
    let dir = tempfile::tempdir().unwrap();
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"></svg>"#;
    std::fs::write(dir.path().join("icon.svg"), svg).unwrap();
    let config = test_config(dir.path().to_str().unwrap());
    let (cfg, client, cache, registry, app_metrics) = build_app_data(config);
    let app = test::init_service(
        App::new()
            .app_data(cfg)
            .app_data(client)
            .app_data(cache)
            .app_data(registry.clone())
            .app_data(app_metrics)
            .service(image_proxy::api::image::process_image_request),
    )
    .await;

    // Query param forces non-pass-through so we hit the non_processable branch
    // (svg with empty query may pass-through — still ok labels differ; use ?bw=1
    // or size so path is non_processable after load).
    let resp = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/icon.svg?size=8")
            .to_request(),
    )
    .await;
    assert_eq!(resp.status(), 200);
    let _ = test::read_body(resp).await;

    let text = gather_text(registry.get_ref());
    assert!(
        has_sample(
            &text,
            "image_requests_total",
            &["status=\"ok\"", "path=\"non_processable\""]
        ),
        "non_processable labels missing: {text}"
    );
}

#[actix_web::test]
async fn handler_labels_cache_hit() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("photo.jpeg"), make_test_jpeg_bytes(32, 32)).unwrap();

    let mut cfg = EncodingConfig {
        root_path: dir.path().to_str().unwrap().to_string(),
        enable_cache: true,
        cache_memory_size: 4 * 1024 * 1024,
        cache_memory_max_item_size: 2 * 1024 * 1024,
        ..EncodingConfig::default()
    };
    cfg.cache_status_header = "X-Image-Proxy-Cache".to_string();
    let (cfg, client, cache, registry, app_metrics) = build_app_data_with_cache(Arc::new(cfg)).await;

    let app = test::init_service(
        App::new()
            .app_data(cfg)
            .app_data(client)
            .app_data(cache)
            .app_data(registry.clone())
            .app_data(app_metrics)
            .service(image_proxy::api::image::process_image_request),
    )
    .await;

    let uri = "/photo.jpeg?format=webp&size=16";

    let resp1 = test::call_service(&app, test::TestRequest::get().uri(uri).to_request()).await;
    assert_eq!(resp1.status(), 200);
    let _ = test::read_body(resp1).await;

    let resp2 = test::call_service(&app, test::TestRequest::get().uri(uri).to_request()).await;
    assert_eq!(resp2.status(), 200);
    assert_eq!(
        resp2
            .headers()
            .get("X-Image-Proxy-Cache")
            .and_then(|v| v.to_str().ok()),
        Some("HIT")
    );
    let _ = test::read_body(resp2).await;

    let text = gather_text(registry.get_ref());
    assert!(
        has_sample(
            &text,
            "image_requests_total",
            &["status=\"ok\"", "path=\"cache_hit\""]
        ),
        "cache_hit labels missing: {text}"
    );
    // First request should still have been a transform miss
    assert!(
        has_sample(
            &text,
            "image_requests_total",
            &["status=\"ok\"", "path=\"transform\""]
        ),
        "transform miss sample missing: {text}"
    );
}
