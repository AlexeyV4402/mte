use std::path::PathBuf;

use serde::Deserialize;

#[allow(unused)]
#[derive(Deserialize, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub listen_address: String,
    pub auth_timeout_secs: u64,
    pub server_timeout_secs: u64,
    pub server_toml_path: PathBuf,
}

impl AppConfig {
    pub fn load() -> Self {
        let content = std::fs::read_to_string("config.toml")
            .unwrap_or_else(|_| panic!("Файл config.toml не найден!"));

        toml::from_str(&content).expect("Ошибка парсинга config.toml")
    }
}
