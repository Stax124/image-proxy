#[macro_use]
mod common;

use std::sync::Arc;

use actix_web::{App, test};
use common::{build_app_data_with_client, make_test_jpeg_bytes, start_user_agent_capturing_server};
use image_proxy::config::EncodingConfig;
use image_proxy::utils::{default_user_agent, resolve_user_agent};

/// Build an in-process app whose outbound HTTP client uses the given
/// `User-Agent` default header, then request a missing file so the fallback
/// path fetches from `upstream`. Returns the captured `User-Agent`.
async fn captured_user_agent_for_client(client: awc::Client) -> String {
    let dir = tempfile::tempdir().unwrap();
    let jpeg = make_test_jpeg_bytes(16, 16);
    let (upstream, captured, _handle) = start_user_agent_capturing_server(jpeg, "image/jpeg").await;

    let config = Arc::new(EncodingConfig {
        root_path: dir.path().to_str().unwrap().to_string(),
        fallback_image_url: Some(upstream),
        ..EncodingConfig::default()
    });

    let (cfg, http, cache, reg, pd, rc) = build_app_data_with_client(config, client);
    let app = test::init_service(
        App::new()
            .app_data(cfg)
            .app_data(http)
            .app_data(cache)
            .app_data(reg)
            .app_data(pd)
            .app_data(rc)
            .service(image_proxy::api::image::process_image_request),
    )
    .await;

    let req = test::TestRequest::get().uri("/missing.jpeg").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "fallback fetch should succeed");
    let _ = test::read_body(resp).await;

    captured
        .lock()
        .unwrap()
        .clone()
        .expect("upstream should have received a User-Agent header")
}

#[actix_web::test]
async fn default_user_agent_is_sent_to_upstream() {
    let client = awc::ClientBuilder::new()
        .add_default_header((awc::http::header::USER_AGENT, resolve_user_agent()))
        .finish();

    let captured = captured_user_agent_for_client(client).await;
    assert_eq!(captured, default_user_agent());
    assert!(captured.starts_with("image-proxy/"));
}

#[actix_web::test]
async fn custom_user_agent_is_sent_to_upstream() {
    let custom = "MyCompany-ImageFetcher/3.0 (+https://example.com/bot)";
    let client = awc::ClientBuilder::new()
        .add_default_header((awc::http::header::USER_AGENT, custom))
        .finish();

    let captured = captured_user_agent_for_client(client).await;
    assert_eq!(captured, custom);
}
