use game_lib::network::types::{PacketC2G, PlayerFullData};

pub enum UdpCommand {
    Send { channel: u8, packet: PacketC2G },
    // SpawnPlayer { client_id: u64 },
    // Kick { client_id: u64 },
}

// pub enum TcpCommand {
//     // Send(PacketG2D),
// }

pub enum ECSCommand {
    Prepare(PlayerFullData),
    Run,
}
