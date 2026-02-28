use crate::database;
use crate::prelude::*;
use rand::rngs::OsRng;
use russh::keys::{Algorithm, PrivateKey, decode_secret_key, encode_pkcs8_pem};
use russh::server::{Auth, Handler};
use russh::{MethodKind, MethodSet, SshId};
use sqlx::SqlitePool;
use std::net::SocketAddr;
use tokio::{fs, task::spawn_blocking};

pub struct HoneypotHandler {
    pub pool: SqlitePool,
    pub connection_id: i64,
    pub peer_addr: SocketAddr,
    pub attempt_number: u32,
}

impl Handler for HoneypotHandler {
    type Error = russh::Error;

    fn auth_password(
        &mut self,
        user: &str,
        password: &str,
    ) -> impl std::future::Future<Output = Result<Auth, Self::Error>> + Send {
        let user = user.to_string();
        let password = password.to_string();
        let pool = self.pool.clone();
        let conn_id = self.connection_id;
        let peer_addr = self.peer_addr;

        self.attempt_number += 1;
        let attempt_num = self.attempt_number;

        async move {
            // Emit trace for login attempt
            info!(
               target: "runtime",
               event = "AuthAttempt",
               client = %peer_addr,
               password = %password,
               username = %user,
               attempt = attempt_num,
            );

            // Record to database
            if let Err(e) =
                database::insert_auth_attempt(&pool, conn_id, attempt_num, &user, &password).await
            {
                error!(
                    client = %peer_addr,
                    error = %e,
                    "Failed to record auth attempt"
                );
            }

            // Allow retries
            let mut methods = MethodSet::empty();
            methods.push(MethodKind::Password);

            Ok(Auth::Reject {
                proceed_with_methods: Some(methods),
                partial_success: false,
            })
        }
    }
}

/// Generate and save SSH host key during init
pub async fn generate_host_key(path: &str) -> anyhow::Result<()> {
    info!("Generating SSH host key at {}", path);

    // Generate ed25519 keypair
    let keypair =
        PrivateKey::random(&mut OsRng, Algorithm::Ed25519).context("Generating SSH Key")?;

    let key_file = std::fs::File::create_new(path).context("Creating KeyFile");

    match key_file {
        Ok(mut file) => {
            // Write Key to File
            spawn_blocking(move || -> Result<()> {
                encode_pkcs8_pem(&keypair, &mut file)?;
                Ok(())
            })
            .await??;

            info!("SSH host key generated successfully");
            Ok(())
        }
        Err(_) => {
            error!("Key Already Exists, ignoring...");
            Ok(())
        }
    }
}

/// Load SSH host key from file and create config
pub async fn load_ssh_config(key_path: &str) -> anyhow::Result<russh::server::Config> {
    info!("Loading SSH host key from {}", key_path);

    // Read key file
    let key_data = fs::read(key_path)
        .await
        .context("Failed to read host key file")?;

    // Decode the key
    let keypair = decode_secret_key(&String::from_utf8_lossy(&key_data), None)
        .context("Failed to decode host key")?;

    let config = russh::server::Config {
        // Mimic a real openssh id
        server_id: SshId::Standard("SSH-2.0-OpenSSH_9.6".to_string()),
        auth_rejection_time: std::time::Duration::from_secs(1),
        max_auth_attempts: 10,
        keys: vec![keypair],
        ..Default::default()
    };

    Ok(config)
}
