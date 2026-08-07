use game_lib::content::types::{PlayerCountType, PlayerIdType};
use game_lib::network::types::{PacketG2C, PlayerFullData, VerifyTokenType};

pub enum UdpCommand {
    Send {
        client_id: u64,
        channel: u8,
        packet: PacketG2C,
    },
    Broadcast {
        channel: u8,
        packet: PacketG2C,
    },
    // SpawnPlayer { client_id: u64 },
    // Kick { client_id: u64 },
}

pub enum TcpCommand {
    Heartbeat(PlayerCountType),
    VerifyToken {
        token: VerifyTokenType,
        client_id: u64,
    },
}

pub enum ECSCommand {
    AddPlayer(PlayerFullData),
    Kick { player_id: PlayerIdType },
}
