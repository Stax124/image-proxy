use std::sync::Arc;

use actix_web::{App, HttpServer, middleware, web};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    EnvFilter,
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

use crate::{api::image::process_image_request, config::EncodingConfig};

mod api;
mod config;
mod operations;
mod utils;

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer().with_span_events(FmtSpan::CLOSE))
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let config = Arc::new(EncodingConfig::from_env());

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(config.clone()))
            .wrap(middleware::Logger::new("%a %r %s %b %D"))
            .service(process_image_request)
    })
    .bind(std::env::var("IMAGE_PROXY_BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8000".to_string()))?
    .run()
    .await?;

    Ok(())
}
