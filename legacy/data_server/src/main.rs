use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use game_lib::utils::common::run_logger;
use tokio::net::TcpListener;
use tracing::{Instrument, info, warn};

mod config;
mod database;
mod network;
mod utils;

use crate::config::AppConfig;
use crate::network::handle::handle_secure;
use crate::utils::data_manager::DataManager;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run_logger(&PathBuf::from("game_server.log"));

    let config = AppConfig::load();
    let server_timeout_secs = config.server_timeout_secs;

    let listener = TcpListener::bind(&config.listen_address)
        .await
        .with_context(|| format!("Сбой на адресе {}", &config.listen_address))?;

    info!(
        "🚀 Сервер инициализирован на {} и готов к работе",
        &config.listen_address
    );

    let manager = Arc::new(DataManager::new(config).await);

    let manager_for_cleanup = Arc::clone(&manager);

    tokio::spawn(async move {
        let mut timer = tokio::time::interval(std::time::Duration::from_secs(server_timeout_secs));

        loop {
            timer.tick().await;

            manager_for_cleanup.update().await;
        }
    });

    loop {
        let (stream, addr) = match listener.accept().await {
            Ok(connection) => connection,
            Err(e) => {
                warn!("Не удалось принять соединение: {:?}", e);

                tokio::time::sleep(std::time::Duration::from_millis(10)).await;

                continue;
            }
        };

        let manager_for_tcp = Arc::clone(&manager);

        let addr_span = tracing::info_span!("session", addr = %addr);
        tokio::spawn(
            async move {
                if let Err(e) = handle_secure(stream, addr, manager_for_tcp).await {
                    warn!("Соединение с {} закрыто: {}", addr, e);
                }
            }
            .instrument(addr_span),
        );
    }
}
