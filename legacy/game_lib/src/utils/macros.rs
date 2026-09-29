#[macro_export]
macro_rules! init_db {
    ($pool:ident) => {
        sqlx::query(
            r#"
            
            CREATE TABLE IF NOT EXISTS players (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                password TEXT NOT NULL,
                balance INTEGER DEFAULT 0,
                faction INTEGER DEFAULT 0,
                map_id INTEGER DEFAULT 1,
                x REAL DEFAULT 0.0,
                y REAL DEFAULT 0.0,
                z REAL DEFAULT 0.0,
                yaw REAL DEFAULT 0.0,
                pitch REAL DEFAULT 0.0,
                inventory BLOB
            );

            CREATE TABLE IF NOT EXISTS mail (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                receiver_id INTEGER NOT NULL,
                sender_name TEXT DEFAULT 'Система',
                subject TEXT,
                message TEXT,
                attachment BLOB,
                is_read INTEGER DEFAULT 0,
                is_claimed INTEGER DEFAULT 0,
                sent_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (receiver_id) REFERENCES players (id) ON DELETE CASCADE
            );
            
            "#
        )
            .execute(&$pool)
            .await
            .expect("Ошибка инициализации таблицы");
    };
}
