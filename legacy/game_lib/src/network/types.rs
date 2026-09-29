use std::net::Ipv4Addr;

use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::config::VersionType;
use crate::content::types::{Faction, PlayerCountType, PlayerIdType};

pub type MsgCounterType = u64;
pub type VerifyTokenType = u64;
pub type ServerIdType = u32;

#[derive(Serialize, Deserialize, Debug, Clone, Encode, Decode)]
pub struct PlayerFullData {
    pub id: PlayerIdType,
    pub name: String,
    pub faction: Faction,
    pub balance: u64,
    pub map_id: u32,
    pub pos: [f32; 3],
    pub rot: [f32; 2],
    pub inventory_raw: Vec<u8>,
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Debug)]
pub enum GameServerAction {
    VerifyToken(VerifyTokenType),
    Heartbeat(PlayerCountType),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuthData {
    pub access_token: VerifyTokenType,
    pub servers: Vec<AuthServerInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuthServerInfo {
    pub ip: Ipv4Addr,
    pub port: u16,
    // pub name: String,
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Debug)]
pub enum PacketC2D {
    // Client to Data-server
    Login {
        user: String,
        pass: String,
        version: VersionType,
    },
    Register {
        user: String,
        pass: String,
    },
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Debug)]
pub enum PacketD2C {
    // Data-server to Client
    AuthResult(Result<AuthData, String>),
    RegisterResult(Result<(), String>),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PacketG2D {
    // Game-server to Data-server
    pub server_id: ServerIdType,
    pub secret: String,
    pub action: GameServerAction,
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Debug)]
pub enum PacketD2G {
    // Data-server to Game-server
    VerificationResult(PlayerFullData),
    AuthFailed(String),
    SaveOk,
}

#[repr(u8)]
#[derive(Encode, Decode, Debug)]
pub enum PacketC2G {
    // Client to Game-server
    // Auth(VerifyTokenType),
    LoadFinished,
}

#[repr(u8)]
#[derive(Encode, Decode, Debug)]
pub enum PacketG2C {
    // Game-server to Client
    AuthSuccess(PlayerFullData),
    JoinAllowed,
    AuthFailed(String),
}

#[derive(Serialize, Deserialize)]
pub enum DataServerIncoming {
    Player(PacketC2D), // Пакеты от клиентов
    Server(PacketG2D), // Пакеты от игровых нод
}

pub trait ToBytes {
    fn to_bytes(&self) -> Vec<u8>;
}

impl<T: serde::Serialize> ToBytes for T {
    fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).expect("Serialization failed")
    }
}

pub trait ToUdpBytes {
    fn to_udp_bytes(&self) -> Vec<u8>;
}

impl<T: bitcode::Encode> ToUdpBytes for T {
    fn to_udp_bytes(&self) -> Vec<u8> {
        bitcode::encode(self)
    }
}
