use crate::prelude::*;
use chrono::Utc;
use sqlx::{Row, Sqlite, SqlitePool, migrate::MigrateDatabase, sqlite::SqlitePoolOptions};
use std::net::IpAddr;

/// Create a new database file and tables
pub async fn create_database(path: String) -> anyhow::Result<()> {
    let db_url = format!("sqlite:{}", path);

    // Check if database exists
    if Sqlite::database_exists(&db_url).await.unwrap_or(false) {
        warn!("Database already exists at {}", path);
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
    let schema = include_str!("schema.sql");
    sqlx::query(schema)
        .execute(&pool)
        .await
        .context("Failed to create tables")?;

    info!("Database created successfully at {}", path);
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
