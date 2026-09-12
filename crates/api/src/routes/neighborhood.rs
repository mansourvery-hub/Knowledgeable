//! Versioned read-only neighborhood endpoint (Brick G1).
//!
//! `GET /v1/graph/neighborhood?concept_id=<uuid>&depth=<0..5>&limit=<1..100>`
//!
//! Thin controller: validates/sanitizes input, delegates to
//! `application::graph_service`, maps outcomes to stable [`crate::error::AppError`].
//! Never touches SQLx directly (layer separation); never unwraps (no panics).

use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

use crate::routes::AppState;

/// Query params. `concept_id` stays `String` so a malformed UUID maps to our
/// stable `validation_failed` (400) envelope instead of Axum's default rejection.
#[derive(Debug, Deserialize)]
pub struct NeighborhoodQuery {
    pub concept_id: String,
    pub depth: Option<u8>,
    pub limit: Option<i64>,
}

pub async fn handle_get_neighborhood(
    State(state): State<AppState>,
    Query(params): Query<NeighborhoodQuery>,
) -> Result<Json<application::graph_service::Neighborhood>, crate::error::AppError> {
    let concept_id_str = params.concept_id.trim();
    if concept_id_str.is_empty() {
        return Err(crate::error::AppError::Validation("concept_id is required".into()));
    }
    let concept_id = uuid::Uuid::parse_str(concept_id_str)
        .map_err(|_| crate::error::AppError::Validation("concept_id must be a UUID".into()))?;

    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| crate::error::AppError::ServiceUnavailable("database unavailable".into()))?;

    // Server-derived learner scope (single-user simulated); never trust client identity.
    let learner_id = application::conversation_service::ensure_default_learner(pool)
        .await
        .map_err(|e| crate::error::AppError::ServiceUnavailable(format!("database error: {e}")))?;

    let graph = application::graph_service::GraphService::new(std::sync::Arc::new(pool.clone()));
    let neighborhood = graph
        .get_neighborhood(concept_id, learner_id, params.depth, params.limit)
        .await
        .map_err(|e| crate::error::AppError::ServiceUnavailable(format!("database error: {e}")))?
        .ok_or_else(|| crate::error::AppError::NotFound("concept not found".into()))?;

    Ok(Json(neighborhood))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    fn test_state(pool: Option<sqlx::SqlitePool>) -> AppState {
        AppState {
            pool,
            llm: std::sync::Arc::new(llm::FakeLlmClient::new("test")),
            version: "test".into(),
        }
    }

    fn status_of(err: crate::error::AppError) -> axum::http::StatusCode {
        err.into_response().status()
    }

    #[tokio::test]
    async fn missing_pool_maps_to_503() {
        let err = handle_get_neighborhood(
            State(test_state(None)),
            Query(NeighborhoodQuery {
                concept_id: uuid::Uuid::new_v4().to_string(),
                depth: None,
                limit: None,
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(status_of(err), axum::http::StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn malformed_uuid_maps_to_400() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        let err = handle_get_neighborhood(
            State(test_state(Some(pool))),
            Query(NeighborhoodQuery { concept_id: "not-a-uuid".into(), depth: None, limit: None }),
        )
        .await
        .unwrap_err();
        assert_eq!(status_of(err), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn missing_concept_maps_to_404() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        let err = handle_get_neighborhood(
            State(test_state(Some(pool))),
            Query(NeighborhoodQuery {
                concept_id: uuid::Uuid::new_v4().to_string(),
                depth: None,
                limit: None,
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(status_of(err), axum::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn returns_typed_neighborhood() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        let learner =
            application::conversation_service::ensure_default_learner(&pool).await.unwrap();
        let a = uuid::Uuid::new_v4();
        let b = uuid::Uuid::new_v4();
        for (id, name) in [(a, "HN-A"), (b, "HN-B")] {
            let node = domain::ConceptNode {
                id,
                canonical_name: name.into(),
                canonical_statement: format!("{name} statement."),
                learner_statement: None,
                world_confidence: 1.0,
                status: domain::ConceptStatus::Active,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            infrastructure::graph_repo::create_concept(&pool, &node).await.unwrap();
        }
        sqlx::query(
            "INSERT INTO concept_relations (id, from_concept_id, to_concept_id, relation_type) VALUES (?, ?, ?, 'dependency')",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(a.to_string())
        .bind(b.to_string())
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO learner_concept_states (learner_id, concept_id, learner_confidence) VALUES (?, ?, ?)",
        )
        .bind(learner.to_string())
        .bind(b.to_string())
        .bind(0.1f32)
        .execute(&pool)
        .await
        .unwrap();

        let out = handle_get_neighborhood(
            State(test_state(Some(pool))),
            Query(NeighborhoodQuery { concept_id: a.to_string(), depth: Some(2), limit: Some(50) }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(out.nodes.len(), 2);
        assert_eq!(out.edges.len(), 1);
        assert!(matches!(out.edges[0].relation_type, domain::RelationType::Dependency));
    }
}
