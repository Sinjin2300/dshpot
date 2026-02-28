use anyhow::Result;
use clap::Parser;
use dshpot::args::Cli;
use dshpot::logging::LoggingConfig;
use dshpot::{app, logging};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Initialize logging (global setup, done once)
    let logging_config: LoggingConfig = cli.logging.clone().into();
    logging::init(&logging_config)?;

    // Run the application and handle errors
    app::run(cli).await?;

    Ok(())
}
