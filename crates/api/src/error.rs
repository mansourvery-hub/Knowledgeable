use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("validation failed")]
    Validation(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("not found")]
    NotFound(String),
    #[error("conflict")]
    Conflict(String),
    #[error("graph mutation rejected")]
    GraphMutationRejected(String),
    #[error("llm provider error")]
    LlmProvider(String),
    #[error("rate limited")]
    RateLimited,
    #[error("internal error")]
    Internal(String),
}

#[derive(Serialize)]
struct ErrorBody {
    code: String,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            Self::Validation(_) => (StatusCode::BAD_REQUEST, "validation_failed"),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            Self::Forbidden => (StatusCode::FORBIDDEN, "forbidden"),
            Self::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
            Self::Conflict(_) => (StatusCode::CONFLICT, "conflict"),
            Self::GraphMutationRejected(_) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "graph_mutation_rejected")
            }
            Self::LlmProvider(_) => (StatusCode::BAD_GATEWAY, "llm_provider_error"),
            Self::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "rate_limited"),
            Self::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        };

        // Never expose raw internal details; map to stable messages
        let message = match self {
            Self::Validation(m) => m,
            Self::NotFound(m) => m,
            Self::Conflict(m) => m,
            Self::GraphMutationRejected(m) => m,
            Self::LlmProvider(_) => "upstream provider error".into(),
            Self::Internal(_) => "internal error".into(),
            Self::Unauthorized => "unauthorized".into(),
            Self::Forbidden => "forbidden".into(),
            Self::RateLimited => "rate limited".into(),
        };

        let body = Json(ErrorBody { code: code.into(), message });
        (status, body).into_response()
    }
}
