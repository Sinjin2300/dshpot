use tracing::Level;
use tracing_subscriber::{Layer, layer::SubscriberExt, util::SubscriberInitExt};

pub struct LoggingConfig {
    pub level: Level,
    pub json_runtime: bool,
}

pub fn init(config: &LoggingConfig) -> anyhow::Result<()> {
    let level = config.level;

    // App health (errors, warnings, init messages) -> stderr
    let app_health_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .compact()
        .with_target(true)
        .with_filter(
            tracing_subscriber::filter::Targets::new()
                .with_target(env!("CARGO_PKG_NAME"), level)
                .with_default(level),
        );

    // Runtime data (SSH attempts, connections) -> stdout
    let runtime_json_layer = if config.json_runtime {
        Some(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stdout)
                .json()
                .with_level(false)
                .with_current_span(false)
                .with_target(false)
                .with_filter(
                    tracing_subscriber::filter::Targets::new()
                        .with_target("runtime", tracing::Level::TRACE),
                ),
        )
    } else {
        None
    };

    let runtime_compact_layer = if !config.json_runtime {
        Some(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stdout)
                .compact()
                .with_target(false)
                .with_filter(
                    tracing_subscriber::filter::Targets::new()
                        .with_target("runtime", tracing::Level::TRACE),
                ),
        )
    } else {
        None
    };

    // Build the registry
    tracing_subscriber::registry()
        .with(app_health_layer)
        .with(runtime_json_layer)
        .with(runtime_compact_layer)
        .try_init()
        .map_err(|e| anyhow::anyhow!("Failed to initialize logging: {}", e))?;

    Ok(())
}
