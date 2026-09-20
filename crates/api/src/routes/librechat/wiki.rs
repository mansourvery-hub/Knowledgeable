//! `GET /api/concepts/:id/wiki` — Personal Knowledge Wiki (M8, Brick G3).
//!
//! Thin controller over `application::WikiService`: validates the id,
//! serves the cached/generated page, and maps outcomes to the stable
//! [`crate::error::AppError`] envelope. Viewing never fails on LLM trouble:
//! generation errors degrade to `503`, and below-mastery concepts report
//! `wiki_not_ready` (404) instead of an empty page.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::routes::AppState;

#[derive(Debug, Deserialize)]
pub struct WikiQuery {
    pub model: Option<String>,
}

pub async fn handle_get_wiki(
    State(state): State<AppState>,
    Path(concept_id_raw): Path<String>,
    Query(query): Query<WikiQuery>,
) -> Result<Json<serde_json::Value>, crate::error::AppError> {
    let concept_id_str = concept_id_raw.trim();
    if concept_id_str.is_empty() {
        return Err(crate::error::AppError::Validation("concept id is required".into()));
    }
    let concept_id = uuid::Uuid::parse_str(concept_id_str)
        .map_err(|_| crate::error::AppError::Validation("concept id must be a UUID".into()))?;

    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| crate::error::AppError::ServiceUnavailable("database unavailable".into()))?;

    let learner_id = application::conversation_service::ensure_default_learner(pool)
        .await
        .map_err(|e| crate::error::AppError::ServiceUnavailable(format!("database error: {e}")))?;

    // Model override reuses M3 dispatch; unknown models fall back to the
    // boot client. No BYOK here: query strings leak into logs, and wiki
    // generation must never see request credentials.
    let plan = application::llm_dispatch::plan_for(
        query.model.as_deref(),
        None,
        &application::llm_dispatch::ProviderKeys::from_env(),
    )
    .map_err(crate::error::AppError::Validation)?;
    let llm = application::llm_dispatch::build_client(&plan, state.llm.clone());
    let model =
        if plan.model.trim().is_empty() { default_wiki_model() } else { plan.model.clone() };

    let wiki =
        application::wiki_service::WikiService::new(std::sync::Arc::new(pool.clone()), llm, model);
    match wiki.get_wiki(learner_id, concept_id).await {
        // Render consistency (chats = wiki): the drawer runs the same
        // markdown pipeline as chat, so the page carries live-derived
        // annotations for exactly the rendered string. Failures/empties
        // degrade to a missing field, mirroring history behavior.
        Ok(page) => {
            let mut value = serde_json::to_value(&page).map_err(|e| {
                crate::error::AppError::ServiceUnavailable(format!("wiki encode failed: {e}"))
            })?;
            if !page.personalized_content.trim().is_empty() {
                let graph = application::graph_service::GraphService::new(std::sync::Arc::new(
                    pool.clone(),
                ));
                match graph.annotate_turn(learner_id, &page.personalized_content, None).await {
                    Ok(annotations) if !annotations.is_empty() => {
                        if let serde_json::Value::Object(ref mut map) = value {
                            map.insert(
                                "concept_annotations".into(),
                                serde_json::to_value(&annotations)
                                    .unwrap_or(serde_json::Value::Null),
                            );
                        }
                    }
                    Ok(_) => {}
                    Err(e) => tracing::warn!(error = %e, "wiki annotation failed"),
                }
            }
            Ok(Json(value))
        }
        Err(application::wiki_service::WikiError::ConceptNotFound) => {
            Err(crate::error::AppError::NotFound("concept not found".into()))
        }
        Err(application::wiki_service::WikiError::NotReady(reason)) => {
            Err(crate::error::AppError::WikiNotReady(reason))
        }
        Err(application::wiki_service::WikiError::GenerationFailed(detail)) => Err(
            crate::error::AppError::ServiceUnavailable(format!("wiki generation failed: {detail}")),
        ),
        Err(application::wiki_service::WikiError::Db(e)) => {
            Err(crate::error::AppError::ServiceUnavailable(format!("database error: {e}")))
        }
    }
}

fn default_wiki_model() -> String {
    std::env::var("GEMINI_MODEL")
        .ok()
        .filter(|m| !m.trim().is_empty())
        .or_else(|| std::env::var("OPENAI_MODEL").ok().filter(|m| !m.trim().is_empty()))
        .unwrap_or_else(|| "tutor-wiki".to_string())
}
