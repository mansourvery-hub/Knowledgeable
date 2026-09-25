//! Per-learner UI settings persistence (M1 pinned-order brick).
//!
//! The PinnedSection merge treats a successful fetch as authoritative, so
//! these handlers are read-your-writes over the settings store: GET returns
//! exactly what the last POST stored (`[]` when nothing was ever stored).

use axum::{extract::State, Json};
use serde_json::Value;

use crate::error::AppError;
use crate::routes::AppState;

async fn pool(state: &AppState) -> Result<&sqlx::SqlitePool, AppError> {
    state.pool.as_ref().ok_or_else(|| AppError::Internal("no db pool".into()))
}

fn storage_error(e: application::settings_service::SettingsError) -> AppError {
    match e {
        application::settings_service::SettingsError::Validation(message) => {
            AppError::Validation(message)
        }
        application::settings_service::SettingsError::Storage(message) => {
            AppError::Internal(message)
        }
    }
}

/// `GET /api/user/settings/pinned-order` — stored Pinned-section order.
pub async fn get_pinned_order(
    State(state): State<AppState>,
) -> Result<Json<Vec<String>>, AppError> {
    let order = application::settings_service::get_pinned_order(pool(&state).await?)
        .await
        .map_err(storage_error)?;
    Ok(Json(order))
}

/// `POST /api/user/settings/pinned-order` — replace the stored order,
/// echoing what was saved. The body must be `{"pinnedOrder": string[]}`.
pub async fn set_pinned_order(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Result<Json<Vec<String>>, AppError> {
    let order = body
        .get("pinnedOrder")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::Validation("pinnedOrder must be an array of strings".into()))?;
    let mut entries = Vec::with_capacity(order.len());
    for entry in order {
        match entry.as_str() {
            Some(key) => entries.push(key.to_string()),
            None => {
                return Err(AppError::Validation("pinnedOrder must be an array of strings".into()));
            }
        }
    }
    let saved = application::settings_service::set_pinned_order(pool(&state).await?, entries)
        .await
        .map_err(storage_error)?;
    Ok(Json(saved))
}
