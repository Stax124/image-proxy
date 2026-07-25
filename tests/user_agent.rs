#[macro_use]
mod common;

use std::sync::Arc;

use common::{make_test_jpeg_bytes, start_user_agent_capturing_server};
use image_proxy::config::EncodingConfig;
use image_proxy::utils::{build_http_client, default_user_agent};

/// Build an in-process app whose outbound HTTP client uses the given
/// `User-Agent`, then request a missing file so the fallback path fetches
/// from `upstream`. Returns the captured `User-Agent`.
async fn captured_user_agent_for(user_agent: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    let jpeg = make_test_jpeg_bytes(16, 16);
    let (upstream, captured_user_agent, _handle) =
        start_user_agent_capturing_server(jpeg, "image/jpeg").await;

    let config = Arc::new(EncodingConfig {
        root_path: dir.path().to_str().unwrap().to_string(),
        fallback_image_url: Some(upstream),
        user_agent: user_agent.to_string(),
        ..EncodingConfig::default()
    });

    let client = build_http_client(user_agent);
    let app = init_test_app!(config, client);

    let req = actix_web::test::TestRequest::get()
        .uri("/missing.jpeg")
        .to_request();
    let resp = actix_web::test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "fallback fetch should succeed");
    let _ = actix_web::test::read_body(resp).await;

    captured_user_agent
        .lock()
        .unwrap()
        .clone()
        .expect("upstream should have received a User-Agent header")
}

#[actix_web::test]
async fn default_user_agent_is_sent_to_upstream() {
    let expected = default_user_agent();
    let captured = captured_user_agent_for(&expected).await;
    assert_eq!(captured, expected);
    assert!(captured.starts_with("image-proxy/"));
}

#[actix_web::test]
async fn custom_user_agent_is_sent_to_upstream() {
    let custom = "MyCompany-ImageFetcher/3.0 (+https://example.com/bot)";
    let captured = captured_user_agent_for(custom).await;
    assert_eq!(captured, custom);
}
