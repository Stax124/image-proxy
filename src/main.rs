use std::sync::Arc;

use actix_web::{App, HttpServer, middleware, web};

use image_proxy::{
    api::image::process_image_request, api::metrics::metrics_handler, config::EncodingConfig,
    utils::build_http_client,
};

#[actix_web::main]
#[hotpath::main]
async fn main() -> anyhow::Result<()> {
    image_proxy::logs::setup_tracing();
    let config = Arc::new(EncodingConfig::from_env());
    let (prometheus_registry, app_metrics) = image_proxy::metrics::setup_metrics();
    let hybrid_cache = image_proxy::cache::setup_cache(&config, &prometheus_registry).await?;

    HttpServer::new(move || {
        let http_client = build_http_client(&config.user_agent);

        App::new()
            .app_data(web::Data::new(http_client))
            .app_data(web::Data::new(config.clone()))
            .app_data(web::Data::new(hybrid_cache.clone()))
            .app_data(web::Data::new(prometheus_registry.clone()))
            .app_data(web::Data::new(app_metrics.clone()))
            .wrap(middleware::Logger::new("%a %r %s %b %D"))
            .service(metrics_handler)
            .service(process_image_request)
    })
    .bind(std::env::var("IMAGE_PROXY_BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8000".to_string()))?
    .run()
    .await?;

    Ok(())
}
