use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use game_lib::config::VERSION;
use game_lib::network::tcp::{TIMEOUT_SECONDS, perform_handshake_client, read_tcp, send_tcp};
use game_lib::network::types::{
    AuthData, DataServerIncoming, PacketC2D, PacketD2C, PacketG2C, ToBytes, ToUdpBytes, VerifyTokenType
};
use game_lib::network::udp::{PING_MARKER, PROTOCOL, UCI, connection_config};
use renet::RenetClient;
use renet_netcode::{ClientAuthentication, NetcodeClientTransport};
use tokio::net::{TcpStream, UdpSocket as AsyncUdpSocket};
use tokio::sync::mpsc;

use crate::config::{DATA_SERVER, LOGIN, PASSWORD};
use crate::network::types::{ECSCommand, UdpCommand};

pub struct NetManager {}

impl NetManager {
    pub async fn try_login() -> anyhow::Result<AuthData> {
        let stream_res = tokio::time::timeout(
            Duration::from_secs(TIMEOUT_SECONDS),
            TcpStream::connect(DATA_SERVER),
        )
        .await;

        let mut verify_stream = match stream_res {
            Ok(Ok(s)) => s,
            Ok(Err(_)) => {
                return Err(anyhow::anyhow!(
                    "Сервер авторизации недоступен. Проверьте интернет или попробуйте позже."
                ));
            }
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "Превышено время ожидания сервера. Возможно, ведутся техработы."
                ));
            }
        };

        println!("Сервер подключён");

        let verify_cipher = perform_handshake_client(&mut verify_stream)
            .await
            .map_err(|_| anyhow::anyhow!("Ошибка защищенного соединения. Перезапустите клиент."))?;

        println!("Соединение подтверждение");

        let mut msg_counter = 0;

        let payload = DataServerIncoming::Player(PacketC2D::Login {
            user: LOGIN.to_string(),
            pass: PASSWORD.to_string(),
            version: VERSION,
        });

        if let Err(_) = send_tcp(
            &mut verify_stream,
            payload.to_bytes(),
            &mut msg_counter,
            &verify_cipher,
        )
        .await
        {
            return Err(anyhow::anyhow!(
                "Не удалось отправить данные. Повторите попытку."
            ));
        }

        let bytes = read_tcp(&mut verify_stream, &verify_cipher)
            .await
            .map_err(|_| anyhow::anyhow!("Связь с сервером прервана при ожидании ответа."))?;

        let res: PacketD2C = bincode::deserialize(&bytes).map_err(|_| {
            anyhow::anyhow!("Получен некорректный ответ от сервера. Обновите клиент.")
        })?;

        match res {
            PacketD2C::AuthResult(r) => match r {
                Ok(data) => Ok(data),
                Err(e) => Err(anyhow::anyhow!("Отказ в доступе: {}", e)),
            },
            PacketD2C::RegisterResult(_) => Err(anyhow::anyhow!(
                "Критическая ошибка протокола. Обратитесь в поддержку."
            )),
        }
    }

    pub async fn try_register() -> anyhow::Result<()> {
        let stream_res = tokio::time::timeout(
            Duration::from_secs(TIMEOUT_SECONDS),
            TcpStream::connect(DATA_SERVER),
        )
        .await;

        let mut verify_stream = match stream_res {
            Ok(Ok(s)) => s,
            Ok(Err(_)) => {
                return Err(anyhow::anyhow!(
                    "Сервер авторизации недоступен. Проверьте интернет или попробуйте позже."
                ));
            }
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "Превышено время ожидания сервера. Возможно, ведутся техработы."
                ));
            }
        };

        println!("Сервер подключён");

        let verify_cipher = perform_handshake_client(&mut verify_stream)
            .await
            .map_err(|_| anyhow::anyhow!("Ошибка защищенного соединения. Перезапустите клиент."))?;

        println!("Соединение подтверждение");

        let mut msg_counter = 0;

        let payload = DataServerIncoming::Player(PacketC2D::Register {
            user: LOGIN.to_string(),
            pass: PASSWORD.to_string(),
        });

        if let Err(_) = send_tcp(
            &mut verify_stream,
            payload.to_bytes(),
            &mut msg_counter,
            &verify_cipher,
        )
        .await
        {
            return Err(anyhow::anyhow!(
                "Не удалось отправить данные. Повторите попытку."
            ));
        }

        let bytes = read_tcp(&mut verify_stream, &verify_cipher)
            .await
            .map_err(|_| anyhow::anyhow!("Связь с сервером прервана при ожидании ответа."))?;

        let res: PacketD2C = bincode::deserialize(&bytes).map_err(|_| {
            anyhow::anyhow!("Получен некорректный ответ от сервера. Обновите клиент.")
        })?;

        match res {
            PacketD2C::RegisterResult(r) => {
                r.map_err(|e| anyhow::anyhow!("Отказано в доступе: {}", e))
            }
            PacketD2C::AuthResult(_) => Err(anyhow::anyhow!(
                "Критическая ошибка протокола. Обратитесь в поддержку."
            )),
        }
    }

    pub async fn check_ping(addr: SocketAddr) -> u32 {
        let socket = AsyncUdpSocket::bind("0.0.0.0:0").await.unwrap();
        let start = Instant::now();

        socket.send_to(&[PING_MARKER], addr).await.ok();

        let mut buf = [0u8; 1];
        if tokio::time::timeout(Duration::from_millis(500), socket.recv_from(&mut buf))
            .await
            .is_ok()
        {
            return start.elapsed().as_millis() as u32;
        }
        999
    }

    pub fn run_udp(
        game_server_addr: SocketAddr,
        token: VerifyTokenType,
        mut udp_rx: mpsc::UnboundedReceiver<UdpCommand>,
        ecs_tx: mpsc::UnboundedSender<ECSCommand>,
    ) {
        std::thread::spawn(move || {
            let start_time = Instant::now();

            let mut client = RenetClient::new(connection_config());

            let client_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
            let socket = UdpSocket::bind(client_addr).unwrap();

            let client_id = 12345u64;
            let protocol_id = PROTOCOL;

            let bytes = token.to_le_bytes(); // или to_be_bytes()

            let mut buffer = [0u8; 256];

            buffer[..8].copy_from_slice(&bytes);

            let authentication = ClientAuthentication::Unsecure {
                protocol_id,
                client_id,
                user_data: Some(buffer),
                server_addr: game_server_addr,
            };

            let mut transport =
                NetcodeClientTransport::new(Duration::ZERO, authentication, socket).unwrap();

            println!("UDP клиент Renet успешно запущен!");

            loop {
                transport
                    .update(Instant::now() - start_time, &mut client)
                    .unwrap();

                if client.is_disconnected() {
                    println!("Клиент отключён");
                    // continue;
                }

                while let Some(msg) = client.receive_message(UCI::RO) {
                    let packet: PacketG2C = match bitcode::decode(&msg) {
                        Ok(p) => p,
                        Err(e) => {
                            println!("🛑 Кривой пакет от {}: {}", client_id, e);
                            continue;
                        }
                    };

                    match packet {
                        PacketG2C::AuthSuccess(player_full_data) => {
                            ecs_tx
                                .send(ECSCommand::Prepare(player_full_data))
                                .expect("❌ ECS поток завершился аварийно!");
                        }
                        PacketG2C::JoinAllowed => {
                            ecs_tx
                                .send(ECSCommand::Run)
                                .expect("❌ ECS поток завершился аварийно!");
                        }
                        PacketG2C::AuthFailed(_) => {
                            panic!("Ошибка аутентификации")
                        }
                    }
                }

                while let Ok(cmd) = udp_rx.try_recv() {
                    match cmd {
                        UdpCommand::Send { packet, channel } => {
                            client.send_message(channel, packet.to_udp_bytes());
                        }
                    }
                }

                transport.send_packets(&mut client).unwrap();
                std::thread::sleep(Duration::from_millis(16));
            }
        });
    }
}
