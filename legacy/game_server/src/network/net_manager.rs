use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use game_lib::content::types::PlayerCountType;
use game_lib::network::tcp::{TIMEOUT_SECONDS, perform_handshake_client, read_tcp, send_tcp};
use game_lib::network::types::{
    DataServerIncoming, GameServerAction, PacketC2G, PacketD2G, PacketG2C, PacketG2D, PlayerFullData, ToBytes, ToUdpBytes, VerifyTokenType
};
use game_lib::network::udp::{PING_MARKER, PONG_MARKER, PROTOCOL, UCI, connection_config};
use renet::{RenetServer, ServerEvent};
use renet_netcode::{NetcodeServerTransport, ServerAuthentication, ServerConfig};
use tokio::net::{TcpStream, UdpSocket as AsyncUdpSocket};
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{error, info, warn};

use crate::config::{DATA_SERVER, ID, IP, MAX_PLAYERS, PORT, SECRET, TCP_S_CD, UDP_S_CD};
use crate::network::types::{ECSCommand, TcpCommand, UdpCommand};

pub struct NetManager {
    pub pending_players: DashMap<u64, PlayerFullData>, // key = ClientId
    pub udp_tx: mpsc::UnboundedSender<UdpCommand>,
}

impl NetManager {
    pub async fn new(udp_tx: mpsc::UnboundedSender<UdpCommand>) -> Self {
        Self {
            pending_players: DashMap::new(),
            udp_tx,
        }
    }

    pub fn run_udp(
        self: Arc<Self>,
        mut udp_rx: mpsc::UnboundedReceiver<UdpCommand>,
        tcp_tx: mpsc::UnboundedSender<TcpCommand>,
        ecs_tx: mpsc::UnboundedSender<ECSCommand>,
    ) {
        std::thread::spawn(move || {
            let nm = Arc::clone(&self);

            let start_time = Instant::now();

            let public_addr = format!("{}:{}", IP, PORT).parse().unwrap();

            let mut server = RenetServer::new(connection_config());
            let socket = UdpSocket::bind(public_addr).expect("Не удалось занять UDP порт");

            let server_config = ServerConfig {
                current_time: Duration::ZERO,
                max_clients: MAX_PLAYERS as usize,
                protocol_id: PROTOCOL,
                public_addresses: vec![public_addr],
                authentication: ServerAuthentication::Unsecure,
            };

            let mut transport = NetcodeServerTransport::new(server_config, socket)
                .expect("Ошибка создания транспорта");

            loop {
                transport
                    .update(Instant::now() - start_time, &mut server)
                    .expect("Ошибка обновления транспорта");

                while let Some(event) = server.get_event() {
                    match event {
                        ServerEvent::ClientConnected { client_id } => {
                            if let Some(user_data) = transport.user_data(client_id) {
                                if let Ok(token) = bitcode::decode::<VerifyTokenType>(&user_data) {
                                    let r =
                                        tcp_tx.send(TcpCommand::VerifyToken { token, client_id });
                                    if let Err(e) = r {
                                        error!("Ошибка отправки в TCP очередь: {}", e);
                                    }
                                }
                            }
                        }
                        ServerEvent::ClientDisconnected {
                            client_id,
                            reason: _,
                        } => {
                            if let Some((_, player_full_data)) =
                                nm.pending_players.remove(&client_id)
                            {
                                let r = ecs_tx.send(ECSCommand::Kick {
                                    player_id: player_full_data.id,
                                });
                                if let Err(e) = r {
                                    error!("Ошибка отправки в ECS очередь: {}", e);
                                }
                            }
                        }
                    }
                }

                for client_id in server.clients_id() {
                    while let Some(msg) = server.receive_message(client_id, UCI::RO) {
                        let packet: PacketC2G = match bitcode::decode(&msg) {
                            Ok(p) => p,
                            Err(e) => {
                                info!("🛑 Кривой пакет от {}: {}", client_id, e);
                                continue;
                            }
                        };

                        match packet {
                            PacketC2G::LoadFinished => {
                                let player_data = match nm.pending_players.remove(&client_id) {
                                    Some((_, data)) => data,
                                    None => {
                                        info!(
                                            "⚠️ Клиент {} прислал Ready без верификации",
                                            client_id
                                        );
                                        continue;
                                    }
                                };

                                if let Err(e) = ecs_tx.send(ECSCommand::AddPlayer(player_data)) {
                                    error!("Ошибка отправки в ECS очередь: {}", e);
                                }

                                if let Err(e) = self.udp_tx.send(UdpCommand::Send {
                                    client_id,
                                    channel: UCI::RO,
                                    packet: PacketG2C::JoinAllowed,
                                }) {
                                    error!("Ошибка отправки в UDP очередь: {}", e);
                                }
                            }
                        }
                    }
                }

                while let Ok(cmd) = udp_rx.try_recv() {
                    match cmd {
                        UdpCommand::Send {
                            client_id,
                            packet,
                            channel,
                        } => {
                            server.send_message(client_id, channel, packet.to_udp_bytes());
                        }
                        UdpCommand::Broadcast { packet, channel } => {
                            server.broadcast_message(channel, packet.to_udp_bytes());
                        }
                    }
                }

                transport.send_packets(&mut server);
                std::thread::sleep(Duration::from_millis(UDP_S_CD));
            }
        });
    }

    pub async fn run_heartbeat(tcp_tx: mpsc::UnboundedSender<TcpCommand>) {
        let players: PlayerCountType = 0;

        let mut timer = interval(Duration::from_secs(TIMEOUT_SECONDS / 2));
        tokio::spawn(async move {
            loop {
                if let Err(e) = tcp_tx.send(TcpCommand::Heartbeat(players)) {
                    error!("Ошибка отправки в TCP очередь: {}", e);
                }
                timer.tick().await;
            }
        });
    }

    pub async fn run_tcp_server(self: Arc<Self>, mut tcp_rx: mpsc::UnboundedReceiver<TcpCommand>) {
        let server_secret = SECRET.to_string();

        let mut tcp_stream = TcpStream::connect(DATA_SERVER)
            .await
            .expect("Дата-сервер недоступен");
        let cipher = perform_handshake_client(&mut tcp_stream)
            .await
            .expect("Handshake failed");

        let mut msg_counter = 0u64;
        tokio::spawn(async move {
            let mut timer = interval(Duration::from_secs(TCP_S_CD));
            loop {
                while let Some(cmd) = tcp_rx.recv().await {
                    match cmd {
                        TcpCommand::Heartbeat(players) => {
                            if let Err(e) = send_tcp(
                                &mut tcp_stream,
                                DataServerIncoming::Server(PacketG2D {
                                    server_id: ID,
                                    secret: server_secret.clone(),
                                    action: GameServerAction::Heartbeat(players),
                                })
                                .to_bytes(),
                                &mut msg_counter,
                                &cipher,
                            )
                            .await
                            {
                                error!("Ошибка отправки TCP: {}", e);
                                break;
                            }
                        }
                        TcpCommand::VerifyToken { token, client_id } => {
                            let data = PacketG2D {
                                server_id: ID,
                                secret: server_secret.clone(),
                                action: GameServerAction::VerifyToken(token),
                            };

                            if let Err(e) = send_tcp(
                                &mut tcp_stream,
                                DataServerIncoming::Server(data).to_bytes(),
                                &mut msg_counter,
                                &cipher,
                            )
                            .await
                            {
                                error!("Ошибка отправки TCP: {}", e);
                                break;
                            };

                            match read_tcp(&mut tcp_stream, &cipher).await {
                                Ok(bytes) => {
                                    if let Ok(response) = bincode::deserialize::<PacketD2G>(&bytes)
                                    {
                                        match response {
                                            PacketD2G::VerificationResult(player_full_data) => {
                                                self.pending_players
                                                    .insert(client_id, player_full_data.clone());
                                                let r = self.udp_tx.send(UdpCommand::Send {
                                                    client_id: client_id,
                                                    channel: UCI::RO,
                                                    packet: PacketG2C::AuthSuccess(
                                                        player_full_data,
                                                    ),
                                                });
                                                if let Err(e) = r {
                                                    warn!("Ошибка отправки в UDP очередь: {}", e);
                                                }
                                            }
                                            PacketD2G::AuthFailed(s) => {
                                                let r = self.udp_tx.send(UdpCommand::Send {
                                                    client_id: client_id,
                                                    channel: UCI::RO,
                                                    packet: PacketG2C::AuthFailed(s),
                                                });
                                                if let Err(e) = r {
                                                    warn!("Ошибка отправки в UDP очередь: {}", e);
                                                }
                                            }
                                            PacketD2G::SaveOk => {}
                                        }
                                    }
                                }
                                Err(e) => warn!("Ошибка процесса верификации: {}", e),
                            }
                        }
                    }
                }
                timer.tick().await;
            }
        });
    }

    pub async fn run_ping_pong() {
        let ping_addr: SocketAddr = format!("{}:{}", IP, PORT + 1).parse().unwrap();
        let ping_socket = AsyncUdpSocket::bind(ping_addr)
            .await
            .expect("Не удалось занять Пинг-порт");

        tokio::spawn(async move {
            let mut buf = [0u8; 1];
            info!("Пинг-агент запущен на порту {}(UDP)", PORT + 1);

            loop {
                if let Ok((1, client_addr)) = ping_socket.recv_from(&mut buf).await {
                    if buf[0] == PING_MARKER {
                        let _ = ping_socket.send_to(&[PONG_MARKER], client_addr).await;
                    }
                }
            }
        });
    }
}
