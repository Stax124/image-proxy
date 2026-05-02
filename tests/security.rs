#[macro_use]
mod common;

use actix_web::test;
use image_proxy::config::EncodingConfig;
use std::sync::Arc;

#[actix_web::test]
async fn directory_traversal_prevented() {
    let dir = tempfile::tempdir().unwrap();
    // Create a file outside the root
    let secret = dir.path().join("secret.jpeg");
    std::fs::write(&secret, b"secret").unwrap();

    // Root is a subdir
    let root = dir.path().join("images");
    std::fs::create_dir_all(&root).unwrap();
    let config = Arc::new(EncodingConfig {
        root_path: root.to_str().unwrap().to_string(),
        ..EncodingConfig::default()
    });
    let app = init_test_app!(config);

    // Attempt directory traversal
    let req = test::TestRequest::get().uri("/../secret.jpeg").to_request();
    let resp = test::call_service(&app, req).await;
    // Should NOT serve the file outside root - either 404 or the path is sanitized
    assert_eq!(resp.status(), 404);
}
