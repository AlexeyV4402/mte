use game_lib::init_db;

use crate::config::AppConfig;

mod config;

#[tokio::main]
async fn main() {
    let config = AppConfig::load();
    let pool = sqlx::SqlitePool::connect(&config.database_url)
        .await
        .expect("DB connection failed");
    init_db!(pool);
}
