use std::{net::IpAddr, path::PathBuf};
pub enum MetricsConfig {
    Prometheus(IpAddr, u16),
    File(PathBuf),
}

pub async fn serve_prometheus(ip: IpAddr, port: u16) -> anyhow::Result<()> {
    todo!()
}
pub async fn file_metrics_loop() -> anyhow::Result<()> {
    todo!()
}
