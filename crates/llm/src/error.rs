use thiserror::Error;

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("provider error: {message}")]
    Provider {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    #[error("timeout")]
    Timeout,
    #[error("rate limited")]
    RateLimited,
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("internal: {0}")]
    Internal(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmErrorKind {
    Provider,
    Timeout,
    RateLimited,
    InvalidRequest,
    Internal,
}
