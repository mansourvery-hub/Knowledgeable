//! Concept search for pickers and the graph explorer (M9 follow-up).
//!
//! Thin controller over `GraphService::find_concepts`: substring match over
//! canonical names and statements, bounded and read-only. Empty queries
//! return `[]` (typeahead-friendly) rather than erroring.

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::routes::AppState;

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ConceptHit {
    pub id: String,
    pub canonical_name: String,
    pub canonical_statement: String,
}

/// Mastered concepts for the wiki browser (Phase 5a, W1).
///
/// Bounded list at or above the shared wiki mastery threshold, weakest-first.
/// `wiki_status` is `ready` (fresh page), `stale` (page flagged by graph
/// churn), or `none` (no page yet — the drawer generates on miss).
/// `truncated` tells the client to fall back to concept search instead of
/// local name filtering.
#[derive(Debug, Deserialize)]
pub struct MasteredQuery {
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct MasteredHit {
    pub id: String,
    pub name: String,
    pub confidence: f32,
    pub wiki_status: &'static str,
}

#[derive(Debug, Serialize)]
pub struct MasteredListResponse {
    pub items: Vec<MasteredHit>,
    pub truncated: bool,
}

pub async fn handle_list_mastered(
    State(state): State<AppState>,
    Query(params): Query<MasteredQuery>,
) -> Result<Json<MasteredListResponse>, crate::error::AppError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| crate::error::AppError::ServiceUnavailable("database unavailable".into()))?;

    let learner_id = application::conversation_service::ensure_default_learner(pool)
        .await
        .map_err(|e| crate::error::AppError::ServiceUnavailable(format!("database error: {e}")))?;

    let limit = params
        .limit
        .unwrap_or(application::graph_service::MAX_MASTERED_LIST_LIMIT)
        .clamp(1, application::graph_service::MAX_MASTERED_LIST_LIMIT);

    let graph = application::graph_service::GraphService::new(std::sync::Arc::new(pool.clone()));
    let list = graph
        .list_mastered_concepts(learner_id, limit)
        .await
        .map_err(|e| crate::error::AppError::ServiceUnavailable(format!("database error: {e}")))?;

    Ok(Json(MasteredListResponse {
        items: list
            .items
            .into_iter()
            .map(|m| MasteredHit {
                id: m.concept.id.to_string(),
                name: m.concept.canonical_name,
                confidence: m.learner_confidence,
                wiki_status: match m.wiki_stale {
                    None => "none",
                    Some(true) => "stale",
                    Some(false) => "ready",
                },
            })
            .collect(),
        truncated: list.truncated,
    }))
}

pub async fn handle_search_concepts(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<ConceptHit>>, crate::error::AppError> {
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| crate::error::AppError::ServiceUnavailable("database unavailable".into()))?;

    let query = params.q.unwrap_or_default();
    if query.trim().is_empty() {
        return Ok(Json(Vec::new()));
    }
    let limit = params.limit.unwrap_or(8).clamp(1, 20);

    let graph = application::graph_service::GraphService::new(std::sync::Arc::new(pool.clone()));
    let concepts = graph
        .find_concepts(query.trim(), limit)
        .await
        .map_err(|e| crate::error::AppError::ServiceUnavailable(format!("database error: {e}")))?;

    Ok(Json(
        concepts
            .into_iter()
            .map(|c| ConceptHit {
                id: c.id.to_string(),
                canonical_name: c.canonical_name,
                canonical_statement: c.canonical_statement,
            })
            .collect(),
    ))
}
