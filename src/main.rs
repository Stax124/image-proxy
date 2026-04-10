use std::{sync::Arc, time::Duration};

use actix_web::{App, HttpServer, middleware, web};

use image_proxy::{
    api::image::process_image_request, api::metrics::metrics_handler, config::EncodingConfig,
};


#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    image_proxy::logs::setup_tracing();
    let config = Arc::new(EncodingConfig::from_env());
    let (prometheus_registry, pipeline_duration, request_count) = image_proxy::metrics::setup_metrics();
    let hybrid_cache = image_proxy::cache::setup_cache(&config, &prometheus_registry).await?;

    HttpServer::new(move || {
        let http_client = awc::ClientBuilder::new()
            .timeout(Duration::from_secs(5))
            .finish();

        App::new()
            .app_data(web::Data::new(http_client))
            .app_data(web::Data::new(config.clone()))
            .app_data(web::Data::new(hybrid_cache.clone()))
            .app_data(web::Data::new(prometheus_registry.clone()))
            .app_data(web::Data::new(pipeline_duration.clone()))
            .app_data(web::Data::new(request_count.clone()))
            .wrap(middleware::Logger::new("%a %r %s %b %D"))
            .service(metrics_handler)
            .service(process_image_request)
    })
    .bind(std::env::var("IMAGE_PROXY_BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8000".to_string()))?
    .run()
    .await?;

    Ok(())
}
