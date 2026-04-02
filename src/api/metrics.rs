use actix_web::{HttpResponse, web};
use prometheus::{Encoder, TextEncoder};

#[actix_web::get("/metrics")]
async fn metrics_handler(registry: web::Data<prometheus::Registry>) -> HttpResponse {
    let encoder = TextEncoder::new();
    let metric_families = registry.gather();

    let mut buffer = Vec::with_capacity(8192);
    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        tracing::error!("Failed to encode metrics: {}", e);
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4; charset=utf-8")
        .body(buffer)
}
