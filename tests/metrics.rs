use actix_web::{App, test, web};
use image_proxy::{api::metrics::metrics_handler, metrics::setup_metrics};

#[actix_web::test]
async fn metrics_endpoint_returns_200() {
    let (registry, _pd, _rc) = setup_metrics();

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
