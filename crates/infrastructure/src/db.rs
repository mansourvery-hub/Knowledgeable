use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use std::str::FromStr;

pub use sqlx::SqlitePool;

pub async fn create_pool(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    // SQLite canonical URL: sqlite:knowledgeable.db or sqlite://knowledgeable.db
    // Enable WAL, foreign_keys, busy_timeout — robust, modern, file-local.
    let opts = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .busy_timeout(std::time::Duration::from_millis(5000));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect_with(opts)
        .await?;

    // Enforce pragmas on pooled connections (connect options handle most, but re-assert).
    sqlx::query("PRAGMA foreign_keys = ON;").execute(&pool).await?;
    sqlx::query("PRAGMA busy_timeout = 5000;").execute(&pool).await?;

    Ok(pool)
}

pub use sqlx::SqlitePool as SqlitePoolType;
