use std::fs;
use std::net::Ipv4Addr;
use std::path::Path;
use std::time::Instant;

use game_lib::content::types::PlayerCountType;
use game_lib::network::types::{AuthServerInfo, ServerIdType};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServerStaticConfig {
    pub id: ServerIdType,
    pub name: String,
    pub ip: Ipv4Addr,
    pub port: u16,
    pub secret: String,
}

pub struct ServerLiveStatus {
    pub config: ServerStaticConfig,
    pub current_players: PlayerCountType,
    pub last_heartbeat: Instant,
}

impl ServerLiveStatus {
    pub fn to_auth_info(&self) -> AuthServerInfo {
        AuthServerInfo {
            ip: self.config.ip,
            port: self.config.port,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct RootConfig {
    pub servers: Vec<ServerStaticConfig>,
}

pub fn load_servers(path: &Path) -> Vec<ServerStaticConfig> {
    let content = fs::read_to_string(path).expect("Не удалось прочитать файл servers.toml");

    let config: RootConfig = toml::from_str(&content).expect("Ошибка в формате TOML файла");

    config.servers
}
