use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use sqlx::SqlitePool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pool: Option<SqlitePool>,
    pub llm: Arc<dyn llm::LlmClient>,
    pub version: String,
    pub streams: librechat::stream_registry::StreamRegistry,
    /// Built web client directory (`apps/web/client/dist`). `None` disables
    /// static serving (dev mode serves Vite instead).
    pub web_dist_dir: Option<std::path::PathBuf>,
    /// Optional shared-secret bearer gate for public deployments
    /// (`PUBLIC_API_TOKEN`). `None`/empty = open (local dev default).
    pub api_token: Option<String>,
    /// Optional per-key chat-turn budget (`CHAT_RATE_LIMIT_PER_MINUTE`).
    /// `None` = unlimited (local dev default).
    pub chat_limiter: Option<ratelimit::RateLimiter>,
}

pub use librechat::stream_registry::{new_registry, StreamRegistry};

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

use crate::conversations as conv;
mod auth;
mod debug_graph;
mod librechat;
mod neighborhood;
mod ratelimit;

pub use ratelimit::{chat_rate_limit, RateLimiter};

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/health/db", get(health_db))
        .route("/v1/health", get(health))
        .route("/debug/concept", post(debug_graph::handle_create_concept))
        .route("/debug/graph", get(debug_graph::handle_get_graph))
        // Phase 6 (Brick G1): versioned read-only neighborhood for graph inspection
        .route("/v1/graph/neighborhood", get(neighborhood::handle_get_neighborhood))
        // M9/T9 adapter alias: Vite proxies `/api/*` to Axum, so the vendored
        // LibreChat client must reach the same handler via `/api/*`. Same
        // controller, same error envelope; `/v1` stays canonical.
        .route("/api/graph/neighborhood", get(neighborhood::handle_get_neighborhood))
        // Phase 1: conversations
        .route("/v1/conversations", get(conv::list_conversations).post(conv::create_conversation))
        .route("/v1/conversations/:id", get(conv::get_conversation))
        .route("/v1/conversations/:id/messages", get(conv::list_messages).post(conv::send_message))
        // LibreChat frontend adapter (/api/*)
        .merge(librechat::routes(state.chat_limiter.clone()))
        // Unknown API paths must stay JSON 404s: the client relies on them
        // for feature detection and empty states. Without these wildcards
        // the SPA fallback below would answer 200 HTML and break parsing
        // (e.g. a projects list containing undefined entries).
        .route("/api/*rest", axum::routing::any(api_not_found))
        .route("/v1/*rest", axum::routing::any(api_not_found))
        .fallback_service(static_service(state.web_dist_dir.clone()))
        // Bearer gate last-as-outermost: sees every request, allowlists
        // health probes, enforces only when `api_token` is configured.
        .layer(axum::middleware::from_fn_with_state(state.api_token.clone(), auth::bearer_gate))
        .with_state(state)
}

/// Static web client (M10): serves the production build with an `index.html`
/// fallback so SPA routes (`/c/...`) resolve. API routes above take
/// precedence — the fallback only sees unmatched paths. A missing build
/// directory yields an explicit 404 hint instead of opaque file errors.
fn static_service(dir: Option<std::path::PathBuf>) -> Router {
    match dir.filter(|d| d.join("index.html").is_file()) {
        Some(dir) => {
            tracing::info!(path = %dir.display(), "serving web client");
            let index = dir.join("index.html");
            Router::new().fallback_service(
                tower_http::services::ServeDir::new(&dir)
                    // SPA shell: missing paths serve index.html with its
                    // own 200 (`fallback`, not `not_found_service`, which
                    // would force a 404 status).
                    .fallback(tower_http::services::ServeFile::new(index)),
            )
        }
        None => {
            tracing::warn!("web client build missing; static serving disabled");
            Router::new().fallback(missing_build)
        }
    }
}

async fn missing_build() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({
            "code": "web_build_missing",
            "message": "web client not built (run npm run build in apps/web)",
        })),
    )
}

/// Stable JSON 404 for unknown API paths (mirrors `AppError::NotFound`).
async fn api_not_found() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({
            "code": "not_found",
            "message": "not found",
        })),
    )
}
