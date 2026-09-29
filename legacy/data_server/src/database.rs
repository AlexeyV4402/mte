use std::str::FromStr;

use game_lib::content::types::{Faction, PlayerIdType};
use game_lib::init_db;
use game_lib::network::types::PlayerFullData;
use num_traits::FromPrimitive;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteQueryResult};

use crate::config::AppConfig;

pub struct SqlDatabase {
    pub pool: SqlitePool,
}

impl SqlDatabase {
    pub async fn new(config: &AppConfig) -> Self {
        let options = SqliteConnectOptions::from_str(&config.database_url)
            .expect("Неверный формат DATABASE_URL")
            .create_if_missing(true)
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .expect("Не удалось подключиться к БД");

        init_db!(pool);

        Self { pool }
    }

    pub async fn get_auth_info(
        &self,
        user: String,
    ) -> anyhow::Result<Option<(PlayerIdType, String)>> {
        let row = sqlx::query!(
            r#"SELECT id as "id!", password as "password!" FROM players WHERE name = ?"#,
            user
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| (row.id as PlayerIdType, row.password)))
    }

    pub async fn add_player(
        &self,
        user: String,
        hashed: String,
        start_faction: u8,
        start_inv: Vec<u8>,
    ) -> anyhow::Result<SqliteQueryResult> {
        let res = sqlx::query!(
            r#"
            INSERT INTO players (name, password, balance, faction, inventory)
            VALUES (?, ?, ?, ?, ?)
            "#,
            user,
            hashed,
            0,
            start_faction,
            start_inv
        )
        .execute(&self.pool)
        .await?;

        Ok(res)
    }

    pub async fn fetch_player_data(
        &self,
        id: PlayerIdType,
    ) -> anyhow::Result<Option<PlayerFullData>> {
        let row = sqlx::query!(
            r#"SELECT id as "id!", name as "name!", balance as "balance!", faction as "faction!", 
            map_id as "map_id!", x as "x!", y as "y!", z as "z!", yaw as "yaw!", 
            pitch as "pitch!", inventory as "inventory!" 
            FROM players WHERE id = ?"#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        Ok(Some(PlayerFullData {
            id: row.id as PlayerIdType,
            name: row.name,
            faction: Faction::from_u8(row.faction as u8).unwrap_or(Faction::Stalker),
            balance: row.balance as u64,
            map_id: row.map_id as u32,
            pos: [row.x as f32, row.y as f32, row.z as f32],
            rot: [row.yaw as f32, row.pitch as f32],
            inventory_raw: row.inventory,
        }))
    }
}
