//! Per-learner UI settings (M1 pinned-order persistence).
//!
//! The PinnedSection merge treats a *successful* pinned-order fetch as
//! authoritative server state, so this store is read-your-writes: GET
//! returns exactly what the last POST stored (or `[]` when nothing was
//! ever stored), and corrupt rows degrade to `[]` rather than failing.

use sqlx::SqlitePool;

const PINNED_ORDER_KEY: &str = "pinned_order";
/// Upper bound on stored entry keys: the client posts the whole arrangement
/// per reorder, so an unbounded array would let one request bloat the row.
pub const MAX_PINNED_ORDER_ENTRIES: usize = 1000;

#[derive(Debug, PartialEq)]
pub enum SettingsError {
    Validation(String),
    Storage(String),
}

/// Read the stored pinned order. Missing keys, non-JSON rows, and JSON
/// that is not a string array all read as empty — the sidebar falls back
/// to its local arrangement either way.
pub async fn get_pinned_order(pool: &SqlitePool) -> Result<Vec<String>, SettingsError> {
    let raw = infrastructure::settings_repo::get_setting(pool, PINNED_ORDER_KEY)
        .await
        .map_err(|e| SettingsError::Storage(e.to_string()))?;
    Ok(raw.and_then(|value| serde_json::from_str::<Vec<String>>(&value).ok()).unwrap_or_default())
}

/// Replace the stored pinned order, echoing what was saved.
pub async fn set_pinned_order(
    pool: &SqlitePool,
    order: Vec<String>,
) -> Result<Vec<String>, SettingsError> {
    if order.len() > MAX_PINNED_ORDER_ENTRIES {
        return Err(SettingsError::Validation(format!(
            "pinned order exceeds {MAX_PINNED_ORDER_ENTRIES} entries"
        )));
    }
    let value = serde_json::to_string(&order).map_err(|e| SettingsError::Storage(e.to_string()))?;
    infrastructure::settings_repo::set_setting(pool, PINNED_ORDER_KEY, &value)
        .await
        .map_err(|e| SettingsError::Storage(e.to_string()))?;
    Ok(order)
}
