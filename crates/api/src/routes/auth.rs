//! Optional shared-secret bearer gate for public deployments.
//!
//! Contract:
//! - `api_token` unset/empty = gate open (local dev default; zero behavior
//!   change, all existing tests run this way).
//! - `api_token` set = every `/api/*`, `/v1/*` (except the health probes),
//!   and `/debug/*` path requires `Authorization: Bearer <token>`,
//!   else 401 with the standard `unauthorized` envelope.
//! - Health probes (`/health`, `/health/db`, `/v1/health`) and the static
//!   web shell / SPA routes stay open: the shell carries no data (all data
//!   flows through gated API), and browsers cannot attach bearer tokens to
//!   document navigation. Gating the shell would brick recovery, not add
//!   security.
//!
//! The web/phone clients do not send this token yet — wiring it into fetch
//! layers is a follow-up brick. Until then, enabling the gate is for
//! curl-gated testers, not end users.

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Exact health-probe paths that bypass the gate (load-balancer friendly).
const OPEN_PATHS: [&str; 3] = ["/health", "/health/db", "/v1/health"];

/// Paths whose responses carry (or mutate) data and must be gated.
fn gated_path(path: &str) -> bool {
    if OPEN_PATHS.contains(&path) {
        return false;
    }
    path == "/api"
        || path.starts_with("/api/")
        || path == "/v1"
        || path.starts_with("/v1/")
        || path == "/debug"
        || path.starts_with("/debug/")
}

/// Constant-time comparison so token checks don't leak prefix information
/// through timing. Length is checked first (length alone is not sensitive).
fn tokens_equal(provided: &str, expected: &str) -> bool {
    let (a, b) = (provided.as_bytes(), expected.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

pub async fn bearer_gate(
    State(expected): State<Option<String>>,
    req: Request,
    next: Next,
) -> Response {
    let Some(expected) = expected.as_deref().filter(|t| !t.is_empty()) else {
        return next.run(req).await;
    };
    if !gated_path(req.uri().path()) {
        return next.run(req).await;
    }
    let authorized = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .is_some_and(|token| tokens_equal(token, expected));
    if authorized {
        next.run(req).await
    } else {
        crate::error::AppError::Unauthorized.into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::{create_router, new_registry, AppState};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    const TOKEN: &str = "test-token-abc123";

    async fn app_with_token(token: Option<&str>) -> axum::Router {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        application::conversation_service::ensure_default_learner(&pool).await.unwrap();
        create_router(AppState {
            pool: Some(pool),
            llm: std::sync::Arc::new(llm::FakeLlmClient::new("test")),
            version: "test".into(),
            streams: new_registry(),
            web_dist_dir: None,
            api_token: token.map(str::to_string),
            chat_limiter: None,
        })
    }

    fn get(uri: &str, token: Option<&str>) -> Request<Body> {
        let mut builder = Request::builder().uri(uri);
        if let Some(t) = token {
            builder = builder.header("authorization", format!("Bearer {t}"));
        }
        builder.body(Body::empty()).unwrap()
    }

    async fn body_code(response: Response) -> (StatusCode, serde_json::Value) {
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[test]
    fn path_scoping() {
        for open in ["/health", "/health/db", "/v1/health", "/", "/c/new", "/api-docs"] {
            assert!(!gated_path(open), "{open} must stay open");
        }
        for gated in [
            "/api",
            "/api/",
            "/api/config",
            "/api/agents/chat/knowledgeable",
            "/v1",
            "/v1/graph/neighborhood",
            "/debug",
            "/debug/graph",
        ] {
            assert!(gated_path(gated), "{gated} must be gated");
        }
    }

    #[tokio::test]
    async fn health_stays_open_when_gate_on() {
        let app = app_with_token(Some(TOKEN)).await;
        for uri in ["/health", "/health/db", "/v1/health"] {
            let response = app.clone().oneshot(get(uri, None)).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{uri}");
        }
    }

    #[tokio::test]
    async fn api_rejects_missing_wrong_and_malformed_tokens() {
        let app = app_with_token(Some(TOKEN)).await;
        let (status, body) =
            body_code(app.clone().oneshot(get("/api/endpoints", None)).await.unwrap()).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["code"], "unauthorized");

        let (status, _) = body_code(
            app.clone().oneshot(get("/api/endpoints", Some("wrong-token"))).await.unwrap(),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);

        // Wrong scheme and bare "Bearer" with no token both fail closed.
        for header in ["Token test-token-abc123", "Bearer", "bearer test-token-abc123"] {
            let req = Request::builder()
                .uri("/api/endpoints")
                .header("authorization", header)
                .body(Body::empty())
                .unwrap();
            let (status, _) = body_code(app.clone().oneshot(req).await.unwrap()).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED, "{header}");
        }
    }

    #[tokio::test]
    async fn correct_token_passes_and_static_shell_stays_open() {
        let app = app_with_token(Some(TOKEN)).await;
        let response = app.clone().oneshot(get("/api/endpoints", Some(TOKEN))).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // No dist build in tests: the static fallback answers 404 here, which
        // proves the request passed THROUGH the gate (else it would be 401).
        let response = app.clone().oneshot(get("/c/new", None)).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn empty_token_means_open() {
        let app = app_with_token(Some("")).await;
        let response = app.clone().oneshot(get("/api/endpoints", None)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
