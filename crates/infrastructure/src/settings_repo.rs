//! Per-learner UI settings blob store (M1 pinned-order persistence).
//!
//! Single row per key; values are JSON encoded by the service layer.
//! Reads of missing keys return `None` (callers degrade to defaults).

use sqlx::SqlitePool;

/// Read a settings value by key. Missing keys are `None`, not an error.
pub async fn get_setting(pool: &SqlitePool, key: &str) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM user_settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|(value,)| value))
}

/// Upsert a settings value by key.
pub async fn set_setting(pool: &SqlitePool, key: &str, value: &str) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO user_settings (key, value) VALUES (?, ?) ON CONFLICT (key) DO UPDATE SET value = excluded.value")
        .bind(key)
        .bind(value)
        .execute(pool)
        .await?;
    Ok(())
}
