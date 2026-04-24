mod common;

use actix_web::{App, test};
use common::build_app_data;
use image_proxy::{api::image::process_image_request, config::EncodingConfig};
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

    // Attempt directory traversal
    let req = test::TestRequest::get().uri("/../secret.jpeg").to_request();
    let resp = test::call_service(&app, req).await;
    // Should NOT serve the file outside root - either 404 or the path is sanitized
    assert_eq!(resp.status(), 404);
}
