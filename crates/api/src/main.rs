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

    let state = api::routes::AppState { pool, version: env!("CARGO_PKG_VERSION").to_string() };

    let router = api::create_router(state);
    let addr: SocketAddr = config.bind_addr().parse()?;

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "listening");

    axum::serve(listener, router).await?;

    Ok(())
}
