use std::{sync::Arc, time::Duration};

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

    tracing::info!(
        "Starting image proxy server version {}",
        env!("CARGO_PKG_VERSION")
    );
    tracing::info!(
        r"
 _                                                             
(_)                                                            
 _ _ __ ___   __ _  __ _  ___        _ __  _ __ _____  ___   _ 
| | '_ ` _ \ / _` |/ _` |/ _ \______| '_ \| '__/ _ \ \/ / | | |
| | | | | | | (_| | (_| |  __/______| |_) | | | (_) >  <| |_| |
|_|_| |_| |_|\__,_|\__, |\___|      | .__/|_|  \___/_/\_\\__, |
                    __/ |           | |                   __/ |
                   |___/            |_|                  |___/ 
    "
    );

    let config = Arc::new(EncodingConfig::from_env());

    HttpServer::new(move || {
        let http_client = awc::ClientBuilder::new()
            .timeout(Duration::from_secs(5))
            .finish();

        App::new()
            .app_data(web::Data::new(http_client))
            .app_data(web::Data::new(config.clone()))
            .wrap(middleware::Logger::new("%a %r %s %b %D"))
            .service(process_image_request)
    })
    .bind(std::env::var("IMAGE_PROXY_BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8000".to_string()))?
    .run()
    .await?;

    Ok(())
}
