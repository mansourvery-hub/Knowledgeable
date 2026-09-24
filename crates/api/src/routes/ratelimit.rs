//! Fixed-window per-key rate limiter for LLM-spend endpoints.
//!
//! Contract:
//! - `None` limiter = open (local dev default; zero behavior change).
//! - `Some` limiter = each identity gets `max_per_minute` chat turns per
//!   rolling 60s window; excess turns get 429 with the standard
//!   `rate_limited` envelope plus a `Retry-After: 60` header.
//! - Identity = bearer token when the request carries one (unspoofable when
//!   the bearer gate is on: invalid tokens 401 before reaching this layer),
//!   else the peer IP. Single shared bucket for unknown peers.
//! - Bounded memory: at most `MAX_KEYS` identities; expired windows are
//!   evicted lazily, and an over-full map fails open (never self-DoS).
//!
//! Currently wired to `POST /api/agents/chat/:endpoint` only — the one path
//! that spends provider money per call.

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    extract::{ConnectInfo, Request, State},
    http::header,
    middleware::Next,
    response::{IntoResponse, Response},
};

const WINDOW: Duration = Duration::from_secs(60);
const MAX_KEYS: usize = 4096;

#[derive(Clone, Debug)]
pub struct RateLimiter {
    max_per_minute: u32,
    hits: Arc<Mutex<HashMap<String, (Instant, u32)>>>,
}

impl RateLimiter {
    pub fn new(max_per_minute: u32) -> Self {
        Self { max_per_minute: max_per_minute.max(1), hits: Arc::new(Mutex::new(HashMap::new())) }
    }

    /// Returns true when the call is allowed (and records it against the key).
    pub fn check(&self, key: &str) -> bool {
        let mut hits = self.hits.lock().unwrap_or_else(|e| e.into_inner());
        if hits.len() > MAX_KEYS {
            let now = Instant::now();
            hits.retain(|_, (start, _)| now.duration_since(*start) < WINDOW);
        }
        let now = Instant::now();
        match hits.get_mut(key) {
            Some((start, count)) if now.duration_since(*start) < WINDOW => {
                if *count >= self.max_per_minute {
                    return false;
                }
                *count += 1;
                true
            }
            _ => {
                hits.insert(key.to_string(), (now, 1));
                true
            }
        }
    }
}

fn identity(req: &Request, peer: Option<ConnectInfo<SocketAddr>>) -> String {
    if let Some(token) = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .filter(|t| !t.is_empty())
    {
        return format!("tok:{token}");
    }
    match peer {
        Some(ConnectInfo(addr)) => format!("ip:{}", addr.ip()),
        None => "ip:unknown".to_string(),
    }
}

pub async fn chat_rate_limit(
    State(limiter): State<Option<RateLimiter>>,
    peer: Option<ConnectInfo<SocketAddr>>,
    req: Request,
    next: Next,
) -> Response {
    let Some(limiter) = limiter else {
        return next.run(req).await;
    };
    if limiter.check(&identity(&req, peer)) {
        next.run(req).await
    } else {
        let mut response = crate::error::AppError::RateLimited.into_response();
        response
            .headers_mut()
            .insert(header::RETRY_AFTER, axum::http::HeaderValue::from_static("60"));
        response
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

    #[test]
    fn allows_burst_then_denies_per_key() {
        let limiter = RateLimiter::new(2);
        assert!(limiter.check("a"));
        assert!(limiter.check("a"));
        assert!(!limiter.check("a"));
        // Independent keys are unaffected.
        assert!(limiter.check("b"));
    }

    #[tokio::test]
    async fn chat_turns_hit_429_past_budget() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        application::conversation_service::ensure_default_learner(&pool).await.unwrap();
        let app = create_router(AppState {
            pool: Some(pool),
            llm: std::sync::Arc::new(llm::FakeLlmClient::new("test")),
            version: "test".into(),
            streams: new_registry(),
            web_dist_dir: None,
            api_token: None,
            chat_limiter: Some(RateLimiter::new(1)),
        });

        let turn = || {
            Request::builder()
                .method("POST")
                .uri("/api/agents/chat/knowledgeable")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "conversationId": "new",
                        "parentMessageId": crate::routes::librechat::NO_PARENT,
                        "text": "hello",
                        "model": "local-tutor",
                    })
                    .to_string(),
                ))
                .unwrap()
        };

        let first = app.clone().oneshot(turn()).await.unwrap();
        assert_eq!(first.status(), StatusCode::OK);

        let limited = app.clone().oneshot(turn()).await.unwrap();
        assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            limited.headers().get(header::RETRY_AFTER).and_then(|v| v.to_str().ok()),
            Some("60")
        );
        let bytes = axum::body::to_bytes(limited.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["code"], "rate_limited");
    }
}
