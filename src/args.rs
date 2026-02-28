use crate::prelude::*;
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::{
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
};
use tracing::Level;

use crate::{logging::LoggingConfig, metrics::MetricsConfig};

#[derive(Parser, Debug)]
#[command(name = "jahoneypot")]
#[command(version = "0.1")]
#[command(
    about = "SSH Honeypot Tool",
    long_about = "SSH Honeypot that logs authentication attempts and gives access to metrics on those attempts."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    #[command(flatten)]
    pub logging: LoggingConfigInput,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Init {
        #[arg(short = 'd', long = "db", default_value = "honeypot.db")]
        database: String,

        #[arg(
            short = 'k',
            long = "host-key",
            default_value = "honeypot_host_key.pem"
        )]
        host_key: String,
    },
    Serve {
        #[arg(short, long, default_value_t = IpAddr::V4(Ipv4Addr::new(0,0,0,0)))]
        bind_ip: IpAddr,

        #[arg(short, long, default_value_t = 2222, value_parser = validate_port)]
        port: u16,

        #[arg(short = 'd', long = "db", default_value = "honeypot.db")]
        database: String,

        #[arg(
            short = 'k',
            long = "host-key",
            default_value = "honeypot_host_key.pem"
        )]
        host_key: String,

        #[command(flatten)]
        metrics: MetricsConfigInput,
    },
}

// Logging
#[derive(Args, Clone, Debug)]
pub struct LoggingConfigInput {
    #[arg(global = true, value_enum, short = 'l', long = "log-level", default_value_t = LogLevel::Warn)]
    pub log_level: LogLevel,

    /// Emit runtime logs to stdout in JSON format (ignores log level), only works on serve
    #[arg(
        global = true,
        short = 'j',
        long = "output-json",
        default_value_t = false
    )]
    pub json_output: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

// Logging config
impl From<LoggingConfigInput> for LoggingConfig {
    fn from(input: LoggingConfigInput) -> Self {
        LoggingConfig {
            level: input.log_level.into(),
            json_runtime: input.json_output,
        }
    }
}

impl From<LogLevel> for Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => Level::TRACE,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Info => Level::INFO,
            LogLevel::Warn => Level::WARN,
            LogLevel::Error => Level::ERROR,
        }
    }
}

// Metrics
#[derive(Args, Debug)]
#[group(requires_all = ["exporter_type"])]
pub struct MetricsConfigInput {
    #[arg(long = "metrics-exporter", value_enum)]
    pub exporter_type: Option<MetricsExporter>,

    #[arg(long = "prom-ip", required_if_eq("exporter_type", "prometheus"))]
    pub prometheus_ip: Option<IpAddr>,

    #[arg(long = "prom-port", required_if_eq("exporter_type", "prometheus"), value_parser = validate_port)]
    pub prometheus_port: Option<u16>,

    #[arg(long = "metrics-file", required_if_eq("exporter_type", "file"))]
    pub file_path: Option<PathBuf>,
}

#[derive(Debug, Clone, ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum MetricsExporter {
    Prometheus,
    File,
}

impl TryFrom<MetricsConfigInput> for Option<MetricsConfig> {
    type Error = anyhow::Error;

    fn try_from(input: MetricsConfigInput) -> anyhow::Result<Option<MetricsConfig>> {
        if let Some(exporter) = input.exporter_type {
            match exporter {
                MetricsExporter::Prometheus => {
                    let ip_addr = input
                        .prometheus_ip
                        .ok_or_else(|| anyhow!("Prometheus bind ip not specified"))?;
                    let prometheus_port = input
                        .prometheus_port
                        .ok_or_else(|| anyhow!("Prometheus port not specified"))?;
                    Ok(Some(MetricsConfig::Prometheus(ip_addr, prometheus_port)))
                }
                MetricsExporter::File => {
                    let file_path = input
                        .file_path
                        .ok_or_else(|| anyhow!("Filepath not specified for Metrics"))?;
                    Ok(Some(MetricsConfig::File(file_path)))
                }
            }
        } else {
            Ok(None)
        }
    }
}

// Validator
fn validate_port(s: &str) -> Result<u16, String> {
    let port: u16 = s
        .parse()
        .map_err(|_| format!("'{}' is not a valid port number", s))?;

    if port < 1024 {
        return Err(format!(
            "Port {} is reserved (requires root). Use a port >= 1024",
            port
        ));
    }

    Ok(port)
}
