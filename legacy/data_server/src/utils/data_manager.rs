use std::net::IpAddr;
use std::time::{Duration, Instant};

use anyhow::Context;
use dashmap::DashMap;
use game_lib::config::{MAX_LOGIN, MAX_PASS, VERSION, VersionType};
use game_lib::content::types::{Faction, PlayerIdType, empty_inventory};
use game_lib::network::tcp::TIMEOUT_SECONDS;
use game_lib::network::types::{
    AuthData, AuthServerInfo, PacketD2C, PacketD2G, ServerIdType, VerifyTokenType
};
use rand::RngCore;
use tracing::error;

use crate::config::{AppConfig, BCRYPT_COST};
use crate::database::SqlDatabase;
use crate::utils::auth::AuthRegistry;
use crate::utils::game_servers::{ServerLiveStatus, ServerStaticConfig, load_servers};

fn generate_token() -> VerifyTokenType {
    let mut key = [0u8; size_of::<VerifyTokenType>()];
    rand::rngs::OsRng.fill_bytes(&mut key);
    VerifyTokenType::from_le_bytes(key)
}

pub struct DataManager {
    pub db: SqlDatabase,
    pub config: AppConfig,

    pub auth_registry: AuthRegistry,
    pub login_limiter: DashMap<IpAddr, Instant>,
    pub reg_limiter: DashMap<IpAddr, Instant>,

    pub exist_servers: DashMap<ServerIdType, ServerStaticConfig>,
    pub online_servers: DashMap<ServerIdType, ServerLiveStatus>,
}

impl DataManager {
    pub async fn new(config: AppConfig) -> Self {
        let sql_db = SqlDatabase::new(&config).await;
        let static_servers = load_servers(&config.server_toml_path);

        let servers: DashMap<ServerIdType, ServerStaticConfig> =
            static_servers.into_iter().map(|s| (s.id, s)).collect();

        Self {
            db: sql_db,
            config,
            auth_registry: AuthRegistry::new(),
            login_limiter: DashMap::new(),
            reg_limiter: DashMap::new(),
            exist_servers: servers,
            online_servers: DashMap::new(),
        }
    }

    pub async fn process_login(
        &self,
        user: String,
        pass: String,
        version: VersionType,
    ) -> PacketD2C {
        if version != VERSION {
            return PacketD2C::AuthResult(Err("Устаревшая версия клиента".into()));
        }

        let auth_result = match self.verify_login(user, pass).await {
            Ok(Some(id)) => id,
            Ok(None) => {
                return PacketD2C::AuthResult(Err("Неверный логин или пароль".into()));
            }
            Err(e) => {
                eprintln!("Ошибка БД во время входа: {}", e);
                return PacketD2C::AuthResult(Err(
                    "Внутренняя ошибка сервера. Попробуйте позже.".into()
                ));
            }
        };

        let servers = self.get_online_servers().await;
        let token = generate_token();
        self.auth_registry.add_token(token, auth_result);

        PacketD2C::AuthResult(Ok(AuthData {
            access_token: token,
            servers,
        }))
    }

    pub async fn verify_login(
        &self,
        user: String,
        pass: String,
    ) -> anyhow::Result<Option<PlayerIdType>> {
        let Some((id, hashed_pass)) = self.db.get_auth_info(user).await? else {
            return Ok(None);
        };

        let is_valid = tokio::task::spawn_blocking(move || bcrypt::verify(pass, &hashed_pass))
            .await
            .context("Критический сбой потока аутентификации")?
            .unwrap_or(false);

        Ok(is_valid.then_some(id))
    }

    pub async fn process_register(&self, user: String, pass: String) -> PacketD2C {
        if user.len() < 5 || pass.len() < 5 || user.len() > MAX_LOGIN || pass.len() > MAX_PASS {
            return PacketD2C::RegisterResult(Err("Неподходящая длина ника и(или) пароля".into()));
        }

        if !user.chars().all(|c| c.is_alphabetic() || c == '_') {
            return PacketD2C::RegisterResult(Err("Недопустимые символы в нике".into()));
        }

        let spawn_res = tokio::task::spawn_blocking(move || bcrypt::hash(pass, BCRYPT_COST)).await;

        let hashed = match spawn_res {
            Ok(Ok(h)) => h,
            Ok(Err(e)) => {
                error!("Ошибка bcrypt: {:?}", e);
                return PacketD2C::RegisterResult(Err(
                    "Не удалось обработать пароль. Попробуйте другой.".into(),
                ));
            }
            Err(e) => {
                error!("Критическая ошибка потока: {:?}", e);
                return PacketD2C::RegisterResult(Err(
                    "Сервер временно перегружен. Повторите попытку через минуту.".into(),
                ));
            }
        };

        match self
            .db
            .add_player(user, hashed, Faction::Stalker as u8, empty_inventory())
            .await
        {
            Ok(_) => PacketD2C::RegisterResult(Ok(())),
            Err(e) => {
                if e.to_string().contains("UNIQUE") {
                    PacketD2C::RegisterResult(Err("Этот ник уже занят".into()))
                } else {
                    eprintln!("Ошибка БД во время регистрации: {}", e);
                    PacketD2C::RegisterResult(Err("Внутренняя ошибка сервера".into()))
                }
            }
        }
    }

    pub async fn process_verify_token(&self, token: VerifyTokenType) -> PacketD2G {
        let timeout = self.config.auth_timeout_secs;
        let user_id = self.auth_registry.verify_and_remove(token, timeout);

        if user_id == 0 {
            return PacketD2G::AuthFailed("Время ожидания истекло. Зайдите в игру снова.".into());
        }

        match self.db.fetch_player_data(user_id).await {
            Ok(Some(data)) => PacketD2G::VerificationResult(data),

            Ok(None) => {
                eprintln!("Токен {} валиден, но персонаж в БД не найден", user_id);
                PacketD2G::AuthFailed("Ошибка данных персонажа. Свяжитесь с администрацией.".into())
            }

            Err(e) => {
                eprintln!("Ошибка БД во время верификации токена: {}", e);
                PacketD2G::AuthFailed("Сервер временно недоступен. Попробуйте позже.".into())
            }
        }
    }

    pub async fn update(&self) {
        let now = Instant::now();
        let timeout = Duration::from_secs(TIMEOUT_SECONDS);

        // println!(
        //             "⚠️ Сервер [{}] (ID: {}) потерял соединение (Timeout)",
        //             server.config.name, server.config.id
        //         );

        self.auth_registry
            .cleanup_expired(self.config.auth_timeout_secs);

        self.reg_limiter
            .retain(|_, last_attempt| now.duration_since(*last_attempt) < timeout);

        self.login_limiter
            .retain(|_, last_attempt| now.duration_since(*last_attempt) < timeout);

        self.online_servers
            .retain(|_, s| now.duration_since(s.last_heartbeat) < timeout);
    }

    async fn get_online_servers(&self) -> Vec<AuthServerInfo> {
        self.online_servers
            .iter()
            .map(|s| s.value().to_auth_info())
            .collect()
    }
}
