use game_lib::content::types::PlayerCountType;
use game_lib::network::types::ServerIdType;

pub(crate) const DATA_SERVER: &str = "127.0.0.1:8081";
pub(crate) const IP: &str = "127.0.0.1";
pub(crate) const PORT: u64 = 7777;
pub(crate) const SECRET: &str = "key_1";
pub(crate) const ID: ServerIdType = 1;
pub(crate) const MAX_PLAYERS: PlayerCountType = 64;

/// TCP server per-iteration cooldown in milliseconds
pub(crate) const TCP_S_CD: u64 = 16;

/// UDP server per-iteration cooldown in milliseconds
pub(crate) const UDP_S_CD: u64 = 16;
