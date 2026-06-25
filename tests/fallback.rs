#[macro_use]
mod common;

use std::sync::Arc;

use actix_web::test;
use common::{make_test_jpeg_bytes, start_simple_http_server};
use image_proxy::config::EncodingConfig;

/// Helper: write nothing to the dir (so every lookup misses and hits fallback).
fn empty_root() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

#[actix_web::test]
async fn fallback_direct_stream_when_no_transform() {
    let dir = empty_root();
    let jpeg = make_test_jpeg_bytes(32, 20);
    let (upstream, _handle) = start_simple_http_server(jpeg.clone(), "image/jpeg").await;

    let config = Arc::new(EncodingConfig {
        root_path: dir.path().to_str().unwrap().to_string(),
        fallback_image_url: Some(upstream.clone()),
        ..EncodingConfig::default()
    });
    let app = init_test_app!(config);

    // No query params -> direct stream of whatever the upstream returned (same format)
    let req = test::TestRequest::get()
        .uri("/valid-image.jpeg")
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
    let body = test::read_body(resp).await;
    assert_eq!(body.to_vec(), jpeg);
}

#[actix_web::test]
async fn fallback_with_transform_runs_pipeline() {
    let dir = empty_root();
    let jpeg = make_test_jpeg_bytes(32, 20);
    let (upstream, _handle) = start_simple_http_server(jpeg, "image/jpeg").await;

    let config = Arc::new(EncodingConfig {
        root_path: dir.path().to_str().unwrap().to_string(),
        fallback_image_url: Some(upstream),
        ..EncodingConfig::default()
    });
    let app = init_test_app!(config);

    // Different format or size forces body() + decode + pipeline
    let req = test::TestRequest::get()
        .uri("/valid-image.jpeg?format=png&size=10")
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
async fn fallback_upstream_non_success_propagates_status() {
    // We simulate by using a server that returns 404 for everything.
    // The crude start_simple_http_server always returns 200. Build a tiny responder here.
    let dir = empty_root();
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    let base = format!("http://{}/", addr);

    let _handle = tokio::spawn(async move {
        loop {
            let (socket, _) = match listener.accept().await {
                Ok(v) => v,
                Err(_) => break,
            };
            let _ = socket.writable().await;
            let resp = b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            let _ = socket.try_write(resp);
        }
    });

    let config = Arc::new(EncodingConfig {
        root_path: dir.path().to_str().unwrap().to_string(),
        fallback_image_url: Some(base),
        ..EncodingConfig::default()
    });
    let app = init_test_app!(config);

    let req = test::TestRequest::get().uri("/ghost.jpeg").to_request();
    let resp = test::call_service(&app, req).await;
    // The handler turns non-success into an InternalError carrying the upstream status.
    assert_eq!(resp.status().as_u16(), 404);
}

#[actix_web::test]
async fn fallback_connect_failure_returns_bad_gateway() {
    let dir = empty_root();
    // Point at a port that is not listening.
    let config = Arc::new(EncodingConfig {
        root_path: dir.path().to_str().unwrap().to_string(),
        fallback_image_url: Some("http://127.0.0.1:1/".to_string()),
        ..EncodingConfig::default()
    });
    let app = init_test_app!(config);

    let req = test::TestRequest::get().uri("/nope.jpeg").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 502);
}
