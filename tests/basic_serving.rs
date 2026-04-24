mod common;

use actix_web::{App, test};
use common::{build_app_data, test_config, write_test_jpeg};
use image_proxy::api::image::process_image_request;

#[actix_web::test]
async fn serves_existing_jpeg_without_query_params() {
    let dir = tempfile::tempdir().unwrap();
    write_test_jpeg(dir.path(), "photo.jpeg");
    let config = test_config(dir.path().to_str().unwrap());
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

    let req = test::TestRequest::get().uri("/photo.jpeg").to_request();
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
}

#[actix_web::test]
async fn returns_not_found_for_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    let config = test_config(dir.path().to_str().unwrap());
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

    let req = test::TestRequest::get()
        .uri("/nonexistent.jpeg")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);
}

#[actix_web::test]
async fn rejects_unsupported_format() {
    let dir = tempfile::tempdir().unwrap();
    let config = test_config(dir.path().to_str().unwrap());
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

    let req = test::TestRequest::get().uri("/photo.bmp").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 415);
}

#[actix_web::test]
async fn rejects_missing_extension() {
    let dir = tempfile::tempdir().unwrap();
    let config = test_config(dir.path().to_str().unwrap());
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

    let req = test::TestRequest::get().uri("/noextension").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 415);
}
