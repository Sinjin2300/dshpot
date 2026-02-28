use crate::prelude::*;
use chrono::Utc;
use sqlx::{Row, Sqlite, SqlitePool, migrate::MigrateDatabase, sqlite::SqlitePoolOptions};
use std::net::IpAddr;

/// Create a new database file and tables
pub async fn create_database(path: String) -> anyhow::Result<()> {
    let db_url = format!("sqlite:{}", path);

    // Check if database exists
    if Sqlite::database_exists(&db_url).await.unwrap_or(false) {
        info!("Database already exists at {}", path);
        return Ok(());
    }

    info!("Creating database at {}", path);

    // Create the database file
    Sqlite::create_database(&db_url)
        .await
        .context("Failed to create database file")?;

    // Connect to it
    let pool = SqlitePool::connect(&db_url)
        .await
        .context("Failed to connect to new database")?;

    // Create tables from schema.sql
    let schema = include_str!("./schema.sql");
    sqlx::query(schema)
        .execute(&pool)
        .await
        .context("Failed to create tables")?;

    Ok(())
}

/// Connect to an existing database
pub async fn connect_database(path: String) -> anyhow::Result<SqlitePool> {
    let db_url = format!("sqlite:{}", path);

    // Check if database exists
    if !Sqlite::database_exists(&db_url).await.unwrap_or(false) {
        return Err(anyhow!(
            "Database does not exist at {}. Run 'init' command first.",
            path
        ));
    }

    info!("Connecting to database at {}", path);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .context("Failed to connect to database")?;

    Ok(pool)
}

/// Record a new connection to the database
pub async fn insert_connection(
    pool: &SqlitePool,
    source_ip: IpAddr,
    source_port: u16,

    destination_ip: IpAddr,
    destination_port: u16,
) -> Result<i64> {
    let timestamp = Utc::now().to_rfc3339();

    let result = sqlx::query(
        "INSERT INTO connections (timestamp, source_ip, source_port, destination_ip, destination_port, connection_success)
         VALUES (?, ?, ?, ?, ?, ?)
         RETURNING id"
    )
    .bind(timestamp)
    .bind(source_ip.to_string())
    .bind(source_port as i32)
    .bind(destination_ip.to_string())
    .bind(destination_port as i32)
    .bind(1)
    .fetch_one(pool)
    .await?;

    let connection_id: i64 = result.get("id");
    Ok(connection_id)
}

/// Record an authentication attempt
pub async fn insert_auth_attempt(
    pool: &SqlitePool,
    connection_id: i64,
    attempt_number: u32,
    username: &str,
    password: &str,
) -> Result<()> {
    let timestamp = Utc::now().to_rfc3339();

    if let Err(e) = sqlx::query(
        "INSERT INTO auth_attempts 
         (connection_id, timestamp, attempt_number, auth_method, username, password, success)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(connection_id)
    .bind(timestamp)
    .bind(attempt_number as i32)
    .bind("password")
    .bind(username)
    .bind(password)
    .bind(0)
    .execute(pool)
    .await
    {
        error!(error = %e, "Failed to insert auth attempt into db");
    }

    Ok(())
}

/// Get metrics for the last 24 hours
pub async fn get_metrics_24h(pool: &SqlitePool) -> Result<Metrics24h> {
    // Total connections
    let connections_row = sqlx::query(
        "SELECT COUNT(*) as count FROM connections 
         WHERE datetime(timestamp) > datetime('now', '-1 day')",
    )
    .fetch_one(pool)
    .await?;
    let connections: i64 = connections_row.get("count");

    // Unique IPs
    let unique_ips_row = sqlx::query(
        "SELECT COUNT(DISTINCT source_ip) as count FROM connections 
         WHERE datetime(timestamp) > datetime('now', '-1 day')",
    )
    .fetch_one(pool)
    .await?;
    let unique_ips: i64 = unique_ips_row.get("count");

    // Total auth attempts
    let auth_attempts_row = sqlx::query(
        "SELECT COUNT(*) as count FROM auth_attempts 
         WHERE datetime(timestamp) > datetime('now', '-1 day')",
    )
    .fetch_one(pool)
    .await?;
    let auth_attempts: i64 = auth_attempts_row.get("count");

    Ok(Metrics24h {
        connections: connections as u64,
        unique_ips: unique_ips as u64,
        auth_attempts: auth_attempts as u64,
    })
}

#[derive(Debug, Clone)]
pub struct Metrics24h {
    pub connections: u64,
    pub unique_ips: u64,
    pub auth_attempts: u64,
}
