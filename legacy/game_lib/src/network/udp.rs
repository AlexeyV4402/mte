use std::time::Duration;

use renet::{ChannelConfig, ConnectionConfig, SendType};

// pub const PROTOCOL_ID: u64 = 0;

/// Default UDP Channel Id:
/// RO - Reliable Ordered;
/// U - Unreliable;
#[allow(non_upper_case_globals, non_snake_case)]
pub mod UCI {
    pub const RO: u8 = 0;
    pub const U: u8 = 1;
}

pub const PING_MARKER: u8 = 254;
pub const PONG_MARKER: u8 = 255;
pub const PROTOCOL: u64 = 0;

pub fn connection_config() -> ConnectionConfig {
    // Определяем конфигурацию каналов один раз
    let channels_config = vec![
        ChannelConfig {
            channel_id: UCI::RO,
            max_memory_usage_bytes: 10 * 1024 * 1024,
            send_type: SendType::ReliableOrdered {
                resend_time: Duration::from_millis(300),
            },
        },
        ChannelConfig {
            channel_id: UCI::U,
            max_memory_usage_bytes: 10 * 1024 * 1024,
            send_type: SendType::Unreliable,
        },
    ];

    ConnectionConfig {
        available_bytes_per_tick: 1024 * 1024, // Лимит байт за один тик (1MB)
        // Указываем одни и те же каналы для обеих сторон
        server_channels_config: channels_config.clone(),
        client_channels_config: channels_config,
        ..Default::default()
    }
}
