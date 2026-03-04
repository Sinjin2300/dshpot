use crate::args::{Cli, Command, MetricsConfigInput, resolve_path};
use crate::database::{connect_database, create_database, insert_connection};
use crate::metrics::{MetricsConfig, ReportWindow, build_report, format_report, serve_prometheus};
use crate::prelude::*;
use crate::ssh::{HoneypotHandler, generate_host_key, load_ssh_config};
use russh::server::run_stream;
use sqlx::{Pool, Sqlite};
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::net::TcpListener;
use tokio::signal::unix::{SignalKind, signal};
use tokio::task::JoinSet;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
/// Run the application with the provided CLI arguments
pub async fn run(cli: Cli) -> anyhow::Result<()> {
    // Execute the command
    match cli.command {
        Command::Init { database, host_key } => {
            // Get the appropriate path for each
            let database = resolve_path(database, "honeypot.db");
            let host_key = resolve_path(host_key, "honeypot_host_key.pem");
            info!(database = %database, host_key = %host_key, "Resolved paths");

            // Initialize db
            create_database(database).await?;

            // Create SSH Host Key
            generate_host_key(&host_key).await?;
        }
        Command::Serve {
            bind_ip,
            port,
            database,
            host_key,
            metrics,
        } => {
            // Get the appropriate path for each
            let database = resolve_path(database, "honeypot.db");
            let host_key = resolve_path(host_key, "honeypot_host_key.pem");
            info!(database = %database, host_key = %host_key, "Resolved paths");

            // Get database handle
            let pool = connect_database(database).await?;

            // Create shutdown token
            let shutdown = CancellationToken::new();

            // Setup metrics
            setup_metrics(metrics, pool.clone(), shutdown.clone()).await?;

            // Start server
            start_server(bind_ip, port, host_key, pool, shutdown.clone()).await?;
        }
    };

    Ok(())
}

pub async fn start_server(
    bind_ip: IpAddr,
    port: u16,
    host_key: String,
    pool: Pool<Sqlite>,
    shutdown: CancellationToken,
) -> anyhow::Result<()> {
    // Bind Tcp Listener
    let listener = TcpListener::bind((bind_ip, port)).await?;

    // Spawn signal handler that triggers shutdown
    let shutdown_trigger = shutdown.clone();
    tokio::spawn(async move {
        let mut sigterm =
            signal(SignalKind::terminate()).expect("Failed to register SIGTERM handler");

        let mut sigint =
            signal(SignalKind::interrupt()).expect("Failed to register SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => info!("Received SIGTERM"),
            _ = sigint.recv()  => info!("Received SIGINT"),
        }

        shutdown_trigger.cancel();
    });

    // Group tasks
    let mut tasks: JoinSet<()> = JoinSet::new();

    // Create the config for the service
    let config = Arc::new(load_ssh_config(&host_key).await?);

    info!(port = %port, bind_ip = %bind_ip, "Starting SSH honeypot server");

    // Accept Loop
    loop {
        tokio::select! {
        result = listener.accept() => {

            // Accept Connection
            let (socket, peer_addr) = result?;


            // Parse out info for connection entry
            let local_addr = socket.local_addr()?;
            let connection_id = insert_connection(
                &pool,
                peer_addr.ip(),
                peer_addr.port(),
                local_addr.ip(),
                local_addr.port(),
            )
            .await?;

            // Create russh handler
            let handler = HoneypotHandler {
                pool: pool.clone(),
                connection_id,
                peer_addr,
                attempt_number: 0,
            };

            // Get hanle to config
            let config_clone = config.clone();

            // Get handle to shutdown token
            let task_shutdown = shutdown.clone();

            // Dispatch to handle this connection
            tasks.spawn(async move {
                tokio::select!{
                    result = run_stream(config_clone, socket, handler) =>{
                        if let Err(e) = result{
                        warn!(
                            ip = %peer_addr.ip(),
                            connection_id,
                            error = %e,
                            "SSH session terminated with error"
                        );
                        }
                    }
                    _ = task_shutdown.cancelled() => {
                        warn!("Connection interrupted due to shutdown");
                    }
                };
            });


            }
            _ = shutdown.cancelled() => {
                warn!("Shutting down...");
                break;
            }
        };
    }

    // Cleanup from shutdown
    tasks.join_all().await;
    pool.close().await;
    Ok(())
}

pub async fn setup_metrics(
    metrics: MetricsConfigInput,
    pool: Pool<Sqlite>,
    shutdown: CancellationToken,
) -> anyhow::Result<()> {
    info!("Parsing Metrics");
    if let Some(metrics) = metrics.try_into()? {
        match metrics {
            MetricsConfig::Prometheus(ip_addr, port) => {
                tokio::spawn(async move {
                    info!(ip_addr = %ip_addr,"Found Prometheus Config");

                    if let Err(e) = serve_prometheus(ip_addr, port).await {
                        error!(error = %e, "Prometheus Exporter Failed to Start");
                    }
                });
            }
            MetricsConfig::File(path_buf) => {
                tokio::spawn(async move {
                    if let Err(e) = file_metrics_loop(path_buf, pool, shutdown, 5).await {
                        error!(error = %e, "Metrics File Exporter Failed");
                    }
                });
            }
        }
    } else {
        warn!("Metrics not configured");
    }
    Ok(())
}

pub async fn file_metrics_loop(
    dir: PathBuf,
    pool: Pool<Sqlite>,
    shutdown: CancellationToken,
    top_n: u32,
) -> anyhow::Result<()> {
    info!("Starting Metrics File Exporter");

    // Default to updating the files every half hour
    let mut ticker = interval(std::time::Duration::from_secs(1800));

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let windows = [
                    (ReportWindow::LastHour,  "metrics_last_hour.txt"),
                    (ReportWindow::Last24h,   "metrics_last_24h.txt"),
                    (ReportWindow::AllTime,   "metrics_all_time.txt"),
                ];

                for (window, filename) in windows {
                    let report = build_report(&pool, window, top_n).await?;
                    let output = format_report(&report);
                    fs::write(dir.join(filename), output).await
                        .context(format!("Failed to write {}", filename))?;
                }
                info!("Metrics Files Updated");
            }
            _ = shutdown.cancelled() => {
                warn!("Metrics file loop shutting down");
                break;
            }
        }
    }

    Ok(())
}
