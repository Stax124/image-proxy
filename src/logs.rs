use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    EnvFilter,
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

pub fn setup_tracing() {
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

    tracing_subscriber::registry()
        .with(fmt::layer().with_span_events(FmtSpan::CLOSE))
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();
}
