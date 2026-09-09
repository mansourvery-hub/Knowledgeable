use infrastructure::{config::AppConfig, db, telemetry};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::from_env()?;
    telemetry::init_tracing_pretty(&config.rust_log);

    tracing::info!(bind_addr = %config.bind_addr(), "starting api server");

    // Attempt DB connection; degrade gracefully if unavailable (health will report degraded)
    let pool = match db::create_pool(&config.database_url).await {
        Ok(p) => {
            tracing::info!("database connected");
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
