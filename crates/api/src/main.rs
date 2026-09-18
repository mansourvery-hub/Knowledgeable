use infrastructure::{config::AppConfig, db, telemetry};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::from_env()?;
    telemetry::init_tracing_pretty(&config.rust_log);

    tracing::info!(bind_addr = %config.bind_addr(), "starting api server");

    // Attempt DB connection; degrade gracefully if unavailable (health will report degraded)
    // SQLite file is auto-created; run migrations automatically for fresh checkout.
    let pool = match db::create_pool(&config.database_url).await {
        Ok(p) => {
            tracing::info!("database connected");
            // Auto-migrate: ensures `sqlite:knowledgeable.db` is usable without manual `sqlx migrate run`.
            // `migrate!` path is relative to `crates/api` manifest dir, so `../../migrations` = repo root.
            match sqlx::migrate!("../../migrations").run(&p).await {
                Ok(()) => tracing::info!("migrations applied"),
                Err(e) => tracing::warn!(error = %e, "migration failed — continuing degraded"),
            }
            Some(p)
        }
        Err(e) => {
            tracing::warn!(error = %e, "database connection failed — starting in degraded mode");
            None
        }
    };

    let llm: std::sync::Arc<dyn llm::LlmClient> = application::tutor_service::default_llm();

    let state = api::routes::AppState {
        pool,
        llm,
        version: env!("CARGO_PKG_VERSION").to_string(),
        streams: api::routes::new_registry(),
        web_dist_dir: Some(
            std::env::var("WEB_DIST_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| std::path::PathBuf::from("apps/web/client/dist")),
        ),
    };

    tracing::info!("router created");
    // Native Linux app sends no Origin header (no CORS). Web dev server on
    // localhost:8080 / 127.0.0.1:8080 needs explicit origins — browsers treat
    // them as different origins.
    let cors = tower_http::cors::CorsLayer::new()
        .allow_origin(tower_http::cors::AllowOrigin::list([
            axum::http::HeaderValue::from_static("http://localhost:8080"),
            axum::http::HeaderValue::from_static("http://127.0.0.1:8080"),
        ]))
        .allow_methods(tower_http::cors::AllowMethods::any())
        .allow_headers(tower_http::cors::AllowHeaders::any())
        .expose_headers([axum::http::header::CONTENT_TYPE, axum::http::header::AUTHORIZATION]);
    let router = api::create_router(state).layer(cors);

    let addr: SocketAddr = config.bind_addr().parse()?;

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "listening");

    axum::serve(listener, router).await?;

    Ok(())
}
