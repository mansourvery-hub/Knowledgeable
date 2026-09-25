//! Settings service tests: pinned-order read-your-writes, degradation, cap.

use sqlx::SqlitePool;

use crate::settings_service::{self, MAX_PINNED_ORDER_ENTRIES};

async fn setup() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
    pool
}

#[tokio::test]
async fn missing_key_reads_empty() {
    let pool = setup().await;
    assert_eq!(settings_service::get_pinned_order(&pool).await.unwrap(), Vec::<String>::new());
}

#[tokio::test]
async fn round_trip_replaces() {
    let pool = setup().await;
    let first = vec!["b".to_string(), "a".to_string()];
    assert_eq!(settings_service::set_pinned_order(&pool, first.clone()).await.unwrap(), first);
    assert_eq!(settings_service::get_pinned_order(&pool).await.unwrap(), first);
    let second = vec!["a".to_string()];
    assert_eq!(settings_service::set_pinned_order(&pool, second.clone()).await.unwrap(), second);
    assert_eq!(settings_service::get_pinned_order(&pool).await.unwrap(), second);
}

#[tokio::test]
async fn corrupt_row_degrades_to_empty() {
    let pool = setup().await;
    infrastructure::settings_repo::set_setting(&pool, "pinned_order", "not-json{{{").await.unwrap();
    assert_eq!(settings_service::get_pinned_order(&pool).await.unwrap(), Vec::<String>::new());
    infrastructure::settings_repo::set_setting(&pool, "pinned_order", "[1,2]").await.unwrap();
    assert_eq!(settings_service::get_pinned_order(&pool).await.unwrap(), Vec::<String>::new());
}

#[tokio::test]
async fn oversized_order_rejected() {
    let pool = setup().await;
    let big = (0..MAX_PINNED_ORDER_ENTRIES + 1).map(|i| format!("k{i}")).collect::<Vec<_>>();
    let err = settings_service::set_pinned_order(&pool, big).await.unwrap_err();
    assert!(matches!(err, crate::settings_service::SettingsError::Validation(_)));
    assert_eq!(settings_service::get_pinned_order(&pool).await.unwrap(), Vec::<String>::new());
}
