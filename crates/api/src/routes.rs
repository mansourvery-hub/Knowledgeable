use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use serde::Serialize;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AppState {
    pub pool: Option<SqlitePool>,
    pub version: String,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub database_connected: bool,
}

async fn health(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let database_connected = if let Some(pool) = &state.pool {
        // lightweight check: try to acquire connection without query if needed;
        // use a simple query with timeout protection at caller.
        // Here we just check pool is present; deeper check in /health/db if needed.
        !pool.is_closed()
    } else {
        false
    };

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
    let Some(pool) = state.pool else {
        return Err(crate::error::AppError::Internal("no pool".into()));
    };

    // Actual DB roundtrip
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
    Router::new()
        .route("/health", get(health))
        .route("/health/db", get(health_db))
        .route("/v1/health", get(health))
        .with_state(state)
}
