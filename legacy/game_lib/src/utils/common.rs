use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerStatus {
    Online,
    Maintenance,
    Offline,
}

pub fn run_logger(file: &Path) {
    let file_appender = tracing_appender::rolling::daily("./logs", file);
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let console_layer = fmt::layer().with_ansi(true).with_target(false);

    let file_layer = fmt::layer().with_writer(non_blocking).with_ansi(false);

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(console_layer)
        .with(file_layer)
        .init();

    info!("Логгирование запущено");
}
