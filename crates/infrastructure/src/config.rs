use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub port: u16,
    pub host: String,
    pub rust_log: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, anyhow::Error> {
        // Load .env if present; ignore errors (e.g., in CI where env is injected)
        let _ = dotenvy::dotenv();

        let database_url =
            env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:knowledgeable.db".to_string());
        let port = env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(3000);
        let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let rust_log = env::var("RUST_LOG").unwrap_or_else(|_| "info,tower_http=debug".to_string());

        Ok(Self { database_url, port, host, rust_log })
    }

    #[must_use]
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
