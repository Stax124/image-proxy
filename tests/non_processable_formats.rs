#[macro_use]
mod common;

use actix_web::test;
use common::test_config;

/// Write a minimal valid SVG to a temp path.
fn write_test_svg(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><rect width="8" height="8" fill="red"/></svg>"#;
    std::fs::write(&path, svg).unwrap();
    path
}

#[actix_web::test]
async fn serves_svg_without_processing() {
    let dir = tempfile::tempdir().unwrap();
    write_test_svg(dir.path(), "icon.svg");
    let config = test_config(dir.path().to_str().unwrap());
    let app = init_test_app!(config);

    let req = test::TestRequest::get().uri("/icon.svg").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "image/svg+xml"
    );
    let body = test::read_body(resp).await;
    // SVG should be passed through unmodified
    assert!(
        std::str::from_utf8(&body).unwrap().contains("<svg"),
        "SVG content should be returned as-is"
    );
}

#[actix_web::test]
async fn non_processable_format_ignores_transform_params() {
    let dir = tempfile::tempdir().unwrap();
    write_test_svg(dir.path(), "icon.svg");
    let config = test_config(dir.path().to_str().unwrap());
    let app = init_test_app!(config);

    // Even with size and format params, SVG should be returned as-is
    let req = test::TestRequest::get()
        .uri("/icon.svg?size=100&format=png")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let body = test::read_body(resp).await;
    assert!(
        std::str::from_utf8(&body).unwrap().contains("<svg"),
        "SVG should be returned unprocessed even with transform query params"
    );
}
