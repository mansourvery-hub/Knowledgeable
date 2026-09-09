use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub database_connected: bool,
}

impl HealthStatus {
    #[must_use]
    pub fn healthy(database_connected: bool) -> Self {
        Self {
            status: if database_connected { "ok".into() } else { "degraded".into() },
            version: env!("CARGO_PKG_VERSION").to_string(),
            database_connected,
        }
    }
}
