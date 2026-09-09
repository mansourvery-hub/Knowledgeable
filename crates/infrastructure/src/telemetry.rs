use tracing_subscriber::{fmt, EnvFilter};

pub fn init_tracing(rust_log: &str) {
    let filter = EnvFilter::try_new(rust_log).unwrap_or_else(|_| EnvFilter::new("info"));

    fmt().with_env_filter(filter).with_target(true).with_thread_ids(false).json().try_init().ok();
}

pub fn init_tracing_pretty(rust_log: &str) {
    let filter = EnvFilter::try_new(rust_log).unwrap_or_else(|_| EnvFilter::new("info"));

    fmt().with_env_filter(filter).with_target(true).try_init().ok();
}
