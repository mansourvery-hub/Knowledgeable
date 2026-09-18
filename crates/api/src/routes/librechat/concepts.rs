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
