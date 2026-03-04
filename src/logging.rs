use tracing::Level;
use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};

pub struct LoggingConfig {
    pub level: Level,
    pub json_runtime: bool,
    pub dont_log_timestamp: bool,
}

pub fn init(config: &LoggingConfig) -> anyhow::Result<()> {
    let level = config.level;
    let dont_log_timestamp = config.dont_log_timestamp;

    let runtime_filter =
        tracing_subscriber::filter::Targets::new().with_target("runtime", Level::TRACE);

    let app_filter = tracing_subscriber::filter::Targets::new()
        .with_target(env!("CARGO_PKG_NAME"), level)
        .with_target("runtime", Level::ERROR)
        .with_default(level);

    // Box the layer so both branches have the same type
    let app_health_layer: Box<dyn Layer<_> + Send + Sync> = if !dont_log_timestamp {
        Box::new(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .compact()
                .with_target(true)
                .with_filter(app_filter),
        )
    } else {
        Box::new(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .compact()
                .with_target(true)
                .without_time()
                .with_filter(app_filter),
        )
    };

    let runtime_layer: Box<dyn Layer<_> + Send + Sync> = if config.json_runtime {
        if !dont_log_timestamp {
            Box::new(
                tracing_subscriber::fmt::layer()
                    .with_writer(std::io::stdout)
                    .json()
                    .with_level(false)
                    .with_current_span(false)
                    .with_target(false)
                    .with_filter(runtime_filter),
            )
        } else {
            Box::new(
                tracing_subscriber::fmt::layer()
                    .with_writer(std::io::stdout)
                    .json()
                    .with_level(false)
                    .with_current_span(false)
                    .with_target(false)
                    .without_time()
                    .with_filter(runtime_filter),
            )
        }
    } else if !dont_log_timestamp {
        Box::new(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stdout)
                .compact()
                .with_target(false)
                .with_filter(runtime_filter),
        )
    } else {
        Box::new(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stdout)
                .compact()
                .with_target(false)
                .without_time()
                .with_filter(runtime_filter),
        )
    };

    tracing_subscriber::registry()
        .with(app_health_layer)
        .with(runtime_layer)
        .try_init()
        .map_err(|e| anyhow::anyhow!("Failed to initialize logging: {}", e))?;

    Ok(())
}
