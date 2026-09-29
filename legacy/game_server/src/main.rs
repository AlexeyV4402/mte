use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use game_lib::utils::common::run_logger;
use tokio::sync::mpsc;

mod config;
mod network;

use crate::network::net_manager::NetManager;
use crate::network::types::{ECSCommand, TcpCommand, UdpCommand};

#[tokio::main]
async fn main() {
    run_logger(&PathBuf::from("game_server.log"));

    let (cmd_udp_tx, cmd_udp_rx) = mpsc::unbounded_channel::<UdpCommand>();
    let (cmd_tcp_tx, cmd_tcp_rx) = mpsc::unbounded_channel::<TcpCommand>();
    let (cmd_ecs_tx, _cmd_ecs_rx) = mpsc::unbounded_channel::<ECSCommand>();

    let manager = Arc::new(NetManager::new(cmd_udp_tx).await);
    Arc::clone(&manager).run_udp(cmd_udp_rx, cmd_tcp_tx.clone(), cmd_ecs_tx.clone());

    NetManager::run_heartbeat(cmd_tcp_tx.clone()).await;
    NetManager::run_tcp_server(Arc::clone(&manager), cmd_tcp_rx).await;
    NetManager::run_ping_pong().await;

    loop {
        std::thread::sleep(Duration::from_secs(5));
    }

    // let udp_tx_for_tokio = cmd_udp_tx.clone();
    // let udp_tx_for_ecs = cmd_udp_tx.clone();

    // let tcp_tx_for_tokio = cmd_tcp_tx.clone();
    // let tcp_tx_for_ecs = cmd_tcp_tx.clone();
}
