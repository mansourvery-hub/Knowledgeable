use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use serde::Serialize;
use sqlx::SqlitePool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pool: Option<SqlitePool>,
    pub llm: Arc<dyn llm::LlmClient>,
    pub version: String,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub database_connected: bool,
}

async fn health(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let database_connected = if let Some(pool) = &state.pool { !pool.is_closed() } else { false };
    let status = if database_connected { "ok" } else { "degraded" };
    (
        StatusCode::OK,
        Json(HealthResponse {
            status: status.into(),
            version: state.version.clone(),
            database_connected,
        }),
    )
}

async fn health_db(
    State(state): State<AppState>,
) -> Result<Json<HealthResponse>, crate::error::AppError> {
    let pool =
        state.pool.clone().ok_or_else(|| crate::error::AppError::Internal("no pool".into()))?;

    let row: Option<(i32,)> = sqlx::query_as("SELECT 1")
        .fetch_optional(&pool)
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;

    let connected = row.is_some();
    Ok(Json(HealthResponse {
        status: if connected { "ok".into() } else { "degraded".into() },
        version: state.version.clone(),
        database_connected: connected,
    }))
}

pub fn create_router(state: AppState) -> Router {
    use crate::conversations as conv;

    Router::new()
        .route("/health", get(health))
        .route("/health/db", get(health_db))
        .route("/v1/health", get(health))
        // Phase 1: conversations
        .route("/v1/conversations", get(conv::list_conversations).post(conv::create_conversation))
        .route("/v1/conversations/:id", get(conv::get_conversation))
        .route("/v1/conversations/:id/messages", get(conv::list_messages).post(conv::send_message))
        .with_state(state)
}
