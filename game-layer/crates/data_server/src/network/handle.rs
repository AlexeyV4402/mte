use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::anyhow;
use game_lib::network::tcp::{perform_handshake_server, read_tcp, send_tcp};
use game_lib::network::types::{
    DataServerIncoming, GameServerAction, MsgCounterType, PacketC2D, PacketD2C, ToBytes
};
use tokio::net::TcpStream;
use tracing::info;

use crate::utils::data_manager::DataManager;
use crate::utils::game_servers::ServerLiveStatus;

pub async fn handle_secure(
    mut stream: TcpStream,
    addr: SocketAddr,
    dm: Arc<DataManager>,
) -> anyhow::Result<()> {
    let cipher = perform_handshake_server(&mut stream).await?;

    let mut msg_counter: MsgCounterType = 0;

    loop {
        let decrypted = read_tcp(&mut stream, &cipher).await?;
        let request: DataServerIncoming = bincode::deserialize(&decrypted)?;
        match request {
            DataServerIncoming::Player(p_packet) => {
                let now = Instant::now();
                let ip = addr.ip();
                match p_packet {
                    PacketC2D::Login {
                        user,
                        pass,
                        version,
                    } => {
                        println!("Login");
                        if let Some(mut last_attempt) = dm.login_limiter.get_mut(&ip) {
                            let elapsed = now.duration_since(*last_attempt);

                            if elapsed < Duration::from_secs(5) {
                                if elapsed < Duration::from_millis(500) {
                                    return Err(anyhow::anyhow!(""));
                                }

                                let res = PacketD2C::AuthResult(Err("Слишком часто".into()));
                                send_tcp(&mut stream, res.to_bytes(), &mut msg_counter, &cipher)
                                    .await?;
                                *last_attempt = now;
                                return Err(anyhow::anyhow!(""));
                            }
                        }
                        dm.login_limiter.insert(ip, now);

                        let res = dm.process_login(user, pass, version).await;
                        send_tcp(&mut stream, res.to_bytes(), &mut msg_counter, &cipher)
                            .await
                            .ok();
                    }
                    PacketC2D::Register { user, pass } => {
                        println!("Register");
                        if let Some(mut last_attempt) = dm.login_limiter.get_mut(&ip) {
                            let elapsed = now.duration_since(*last_attempt);

                            if elapsed < Duration::from_secs(5) {
                                if elapsed < Duration::from_millis(500) {
                                    return Err(anyhow::anyhow!(""));
                                }

                                let res = PacketD2C::AuthResult(Err("Слишком часто".into()));
                                send_tcp(&mut stream, res.to_bytes(), &mut msg_counter, &cipher)
                                    .await
                                    .ok();

                                *last_attempt = now;
                                return Err(anyhow::anyhow!(""));
                            }
                        }
                        dm.reg_limiter.insert(ip, now);

                        let res = dm.process_register(user, pass).await;
                        send_tcp(&mut stream, res.to_bytes(), &mut msg_counter, &cipher)
                            .await
                            .ok();
                    }
                }
            }

            DataServerIncoming::Server(srv_req) => {
                let is_trusted = dm.exist_servers.iter().any(|entry| {
                    entry.value().id == srv_req.server_id
                        && entry.value().ip == addr.ip()
                        && entry.value().secret == srv_req.secret
                });

                if !is_trusted {
                    return Err(anyhow::anyhow!(
                        "🛑 Попытка доступа от недоверенного сервера: {}",
                        addr
                    ));
                }

                match srv_req.action {
                    GameServerAction::VerifyToken(token) => {
                        println!("VerifyToken");
                        let res = dm.process_verify_token(token).await;
                        send_tcp(&mut stream, res.to_bytes(), &mut msg_counter, &cipher)
                            .await
                            .ok();
                    }
                    GameServerAction::Heartbeat(players) => {
                        println!("Heartbeat");
                        if let Some(mut srv_guard) = dm.online_servers.get_mut(&srv_req.server_id) {
                            srv_guard.last_heartbeat = std::time::Instant::now();
                            srv_guard.current_players = players;
                            drop(srv_guard);
                            for r in dm.online_servers.iter() {
                                info!(
                                    "Сервер: ID={}, Игроков={}",
                                    r.key(),
                                    r.value().current_players
                                );
                            }
                        } else {
                            let config =
                                if let Some(server) = dm.exist_servers.get(&srv_req.server_id) {
                                    server.clone()
                                } else {
                                    return Err(anyhow!("Неизвестный сервер"));
                                };

                            let new_server = ServerLiveStatus {
                                config,
                                current_players: players,
                                last_heartbeat: Instant::now(),
                            };
                            info!("Игровой сервер подключён");
                            dm.online_servers.insert(srv_req.server_id, new_server);
                        }
                    }
                }
            }
        }
        msg_counter += 1;
    }
}
