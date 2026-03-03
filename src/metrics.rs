use std::{net::IpAddr, path::PathBuf};

use chrono::{DateTime, Utc};
use comfy_table::{Table, presets::UTF8_FULL};
use sqlx::SqlitePool;
pub enum MetricsConfig {
    Prometheus(IpAddr, u16),
    File(PathBuf),
}

pub async fn serve_prometheus(_ip: IpAddr, _port: u16) -> anyhow::Result<()> {
    todo!()
}

// Metrics reports
pub struct MetricsReport {
    pub generated_at: DateTime<Utc>,
    pub window: ReportWindow,
    pub connections: u64,
    pub unique_ips: u64,
    pub auth_attempts: u64,
    pub top_usernames: Vec<(String, u64)>,
    pub top_passwords: Vec<(String, u64)>,
    pub top_credential_pairs: Vec<(String, String, u64)>,
    pub top_source_ips: Vec<(String, u64)>,
}

pub enum ReportWindow {
    AllTime,
    Last24h,
    LastHour,
}

impl ReportWindow {
    pub fn since(&self) -> Option<DateTime<Utc>> {
        match self {
            ReportWindow::AllTime => None,
            ReportWindow::Last24h => Some(Utc::now() - chrono::Duration::hours(24)),
            ReportWindow::LastHour => Some(Utc::now() - chrono::Duration::hours(1)),
        }
    }
}

// Format report
pub fn format_report(report: &MetricsReport) -> String {
    let mut out = String::new();

    let window_label = match report.window {
        ReportWindow::AllTime => "All Time",
        ReportWindow::Last24h => "Last 24 Hours",
        ReportWindow::LastHour => "Last Hour",
    };

    out.push_str(&format!(
        "dshpot Report — {} — Generated {}\n\n",
        window_label,
        report.generated_at.format("%Y-%m-%d %H:%M:%S UTC")
    ));

    // Summary table
    out.push_str("Meta Metrics\n");
    let mut summary = Table::new();
    summary.load_preset(UTF8_FULL);
    summary.set_header(vec!["Metric", "Count"]);
    summary.add_row(vec![
        &"Connections".to_string(),
        &report.connections.to_string(),
    ]);
    summary.add_row(vec![
        &"Unique Source IPs".to_string(),
        &report.unique_ips.to_string(),
    ]);
    summary.add_row(vec![
        &"Auth Attempts".to_string(),
        &report.auth_attempts.to_string(),
    ]);
    out.push_str(&summary.to_string());
    out.push_str("\n\n");

    // Top usernames
    out.push_str("Top Usernames\n");
    let mut t = Table::new();
    t.load_preset(UTF8_FULL);
    t.set_header(vec!["Username", "Count"]);
    for (username, count) in &report.top_usernames {
        t.add_row(vec![username, &count.to_string()]);
    }
    out.push_str(&t.to_string());
    out.push_str("\n\n");

    // Top passwords
    out.push_str("Top Passwords\n");
    let mut t = Table::new();
    t.load_preset(UTF8_FULL);
    t.set_header(vec!["Password", "Count"]);
    for (password, count) in &report.top_passwords {
        t.add_row(vec![password, &count.to_string()]);
    }
    out.push_str(&t.to_string());
    out.push_str("\n\n");

    // Top credential pairs
    out.push_str("Top Credential Pairs\n");
    let mut t = Table::new();
    t.load_preset(UTF8_FULL);
    t.set_header(vec!["Username", "Password", "Count"]);
    for (username, password, count) in &report.top_credential_pairs {
        t.add_row(vec![username, password, &count.to_string()]);
    }
    out.push_str(&t.to_string());
    out.push_str("\n\n");

    // Top source ips
    out.push_str("Top Source Ips\n");
    let mut t = Table::new();
    t.load_preset(UTF8_FULL);
    t.set_header(vec!["Source Ip", "Count"]);
    for (source_ip, count) in &report.top_source_ips {
        t.add_row(vec![source_ip, &count.to_string()]);
    }
    out.push_str(&t.to_string());
    out.push_str("\n\n");

    out
}

// Make report
pub async fn build_report(
    pool: &SqlitePool,
    window: ReportWindow,
    top_n: u32,
) -> anyhow::Result<MetricsReport> {
    let since = window.since();
    Ok(MetricsReport {
        generated_at: Utc::now(),
        window,
        connections: count_connections(pool, since).await?,
        unique_ips: count_unique_ips(pool, since).await?,
        auth_attempts: count_auth_attempts(pool, since).await?,
        top_usernames: top_usernames(pool, top_n, since).await?,
        top_passwords: top_passwords(pool, top_n, since).await?,
        top_credential_pairs: top_credential_pairs(pool, top_n, since).await?,
        top_source_ips: top_source_ips(pool, top_n, since).await?,
    })
}

// Auth Attempts
pub async fn count_auth_attempts(
    pool: &SqlitePool,
    since: Option<DateTime<Utc>>,
) -> anyhow::Result<u64> {
    let count = sqlx::query_scalar::<_, u64>(
        r#"
        SELECT COUNT(*)
        FROM auth_attempts
        WHERE (? IS NULL OR timestamp > ?)
        "#,
    )
    .bind(since)
    .bind(since)
    .fetch_one(pool)
    .await?;

    Ok(count)
}

// Unique Ip Count
pub async fn count_unique_ips(
    pool: &SqlitePool,
    since: Option<DateTime<Utc>>,
) -> anyhow::Result<u64> {
    let count = sqlx::query_scalar::<_, u64>(
        r#"
        SELECT COUNT(DISTINCT source_ip)
        FROM connections
        WHERE (? IS NULL OR timestamp > ?)
        "#,
    )
    .bind(since)
    .bind(since)
    .fetch_one(pool)
    .await?;

    Ok(count)
}

// Connections
pub async fn count_connections(
    pool: &SqlitePool,
    since: Option<DateTime<Utc>>,
) -> anyhow::Result<u64> {
    let count = sqlx::query_scalar::<_, u64>(
        r#"
        SELECT COUNT(*)
        FROM connections
        WHERE (? IS NULL OR timestamp > ?)
        "#,
    )
    .bind(since)
    .bind(since)
    .fetch_one(pool)
    .await?;

    Ok(count)
}

// Top N most common usernames
pub async fn top_usernames(
    pool: &SqlitePool,
    limit: u32,
    since: Option<DateTime<Utc>>,
) -> anyhow::Result<Vec<(String, u64)>> {
    let rows = sqlx::query_as::<_, (String, u64)>(
        r#"
        SELECT username, COUNT(*) as count
        FROM auth_attempts
        WHERE (? IS NULL OR timestamp > ?)
        GROUP BY username
        ORDER BY count DESC
        LIMIT ?
        "#,
    )
    .bind(since)
    .bind(since)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

// Top N most common passwords
pub async fn top_passwords(
    pool: &SqlitePool,
    limit: u32,
    since: Option<DateTime<Utc>>,
) -> anyhow::Result<Vec<(String, u64)>> {
    let rows = sqlx::query_as::<_, (String, u64)>(
        r#"
        SELECT password, COUNT(*) as count
        FROM auth_attempts
        WHERE (? IS NULL OR timestamp > ?)
        GROUP BY password
        ORDER BY count DESC
        LIMIT ?
        "#,
    )
    .bind(since)
    .bind(since)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

// Top N most common username/password tuples
pub async fn top_credential_pairs(
    pool: &SqlitePool,
    limit: u32,
    since: Option<DateTime<Utc>>,
) -> anyhow::Result<Vec<(String, String, u64)>> {
    let rows = sqlx::query_as::<_, (String, String, u64)>(
        r#"
        SELECT username, password, COUNT(*) as count
        FROM auth_attempts
        WHERE (? IS NULL OR timestamp > ?)
        GROUP BY username, password
        ORDER BY count DESC
        LIMIT ?
        "#,
    )
    .bind(since)
    .bind(since)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

// Top N source IPs
pub async fn top_source_ips(
    pool: &SqlitePool,
    limit: u32,
    since: Option<DateTime<Utc>>,
) -> anyhow::Result<Vec<(String, u64)>> {
    let rows = sqlx::query_as::<_, (String, u64)>(
        r#"
        SELECT source_ip, COUNT(*) as count
        FROM connections
        WHERE (? IS NULL OR timestamp > ?)
        GROUP BY source_ip
        ORDER BY count DESC
        LIMIT ?
        "#,
    )
    .bind(since)
    .bind(since)
    .bind(limit as i64)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}
