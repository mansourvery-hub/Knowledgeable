//! Conversation and message endpoints for the LibreChat client.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use super::{conv_json, derive_title, msg_json, NO_PARENT};
use crate::error::AppError;
use crate::routes::AppState;

fn parse_uuid(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|_| AppError::Validation(format!("invalid id: {raw}")))
}

async fn pool(state: &AppState) -> Result<&sqlx::SqlitePool, AppError> {
    state.pool.as_ref().ok_or_else(|| AppError::Internal("no db pool".into()))
}

/// `GET /api/convos` — paginated conversation list.
///
/// The client sends `cursor`/`limit`; v1 returns every conversation for the
/// default learner with a null cursor (single-user, small histories).
pub async fn list(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let pool = pool(&state).await?;
    let convs = application::conversation_service::list_conversations(pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let conversations: Vec<Value> = convs.iter().map(conv_json).collect();
    Ok(Json(json!({
        "conversations": conversations,
        "nextCursor": Value::Null,
    })))
}

/// `GET /api/convos/:id`
pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let pool = pool(&state).await?;
    let conversation_id = parse_uuid(&id)?;
    let conv = application::conversation_service::get_conversation(pool, conversation_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("conversation not found".into()))?;
    Ok(Json(conv_json(&conv)))
}

/// `GET /api/convos/gen_title/:id` — returns (and lazily derives) a title.
pub async fn gen_title(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let pool = pool(&state).await?;
    let conversation_id = parse_uuid(&id)?;
    let conv = application::conversation_service::get_conversation(pool, conversation_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("conversation not found".into()))?;

    if let Some(title) = conv.title.clone().filter(|t| !t.trim().is_empty()) {
        return Ok(Json(json!({ "title": title })));
    }

    let messages = application::conversation_service::list_messages(pool, conversation_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let first_user = messages
        .iter()
        .find(|m| matches!(m.role, domain::MessageRole::User))
        .map(|m| m.content.as_str())
        .unwrap_or("");
    let title = derive_title(first_user);

    application::conversation_service::update_conversation_title(pool, conversation_id, &title)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(json!({ "title": title })))
}

#[derive(Deserialize)]
pub struct UpdateRequest {
    pub arg: UpdateArg,
}

#[derive(Deserialize)]
pub struct UpdateArg {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    #[serde(default)]
    pub title: Option<String>,
}

/// `POST /api/convos/update` — persists conversation metadata (title).
pub async fn update(
    State(state): State<AppState>,
    Json(body): Json<UpdateRequest>,
) -> Result<Json<Value>, AppError> {
    let pool = pool(&state).await?;
    let conversation_id = parse_uuid(&body.arg.conversation_id)?;
    let conv = application::conversation_service::get_conversation(pool, conversation_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("conversation not found".into()))?;

    if let Some(title) = body.arg.title.as_deref().filter(|t| !t.trim().is_empty()) {
        application::conversation_service::update_conversation_title(pool, conversation_id, title)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
    }

    let updated = application::conversation_service::get_conversation(pool, conversation_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .unwrap_or(conv);
    Ok(Json(conv_json(&updated)))
}

#[derive(Deserialize)]
pub struct DeleteRequest {
    pub arg: Option<DeleteArg>,
}

#[derive(Deserialize)]
pub struct DeleteArg {
    #[serde(rename = "conversationId")]
    pub conversation_id: Option<String>,
}

/// `DELETE /api/convos` — deletes a conversation and its messages.
pub async fn delete(
    State(state): State<AppState>,
    body: Option<Json<DeleteRequest>>,
) -> Result<Json<Value>, AppError> {
    let pool = pool(&state).await?;
    let conversation_id = body
        .and_then(|Json(b)| b.arg)
        .and_then(|a| a.conversation_id)
        .ok_or_else(|| AppError::Validation("conversationId is required".into()))?;
    let conversation_id = parse_uuid(&conversation_id)?;

    application::conversation_service::delete_conversation(pool, conversation_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(json!({ "acknowledged": true })))
}

/// `GET /api/messages/:conversationId` — message history as `TMessage[]`.
///
/// The schema stores messages linearly; the adapter synthesizes the
/// `parentMessageId` chain LibreChat's message tree expects.
///
/// F7: highlighting is a function of the CURRENT graph, so assistant
/// messages carry live-derived `concept_annotations` (same matcher as the
/// turn-end SSE frame). History badges therefore track mastery instead of
/// going stale. Derivation never breaks history: failures degrade to a
/// missing field, exactly like a missing live frame.
pub async fn list_messages(
    State(state): State<AppState>,
    Path(conversation_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let pool = pool(&state).await?;
    let conversation_id = parse_uuid(&conversation_id)?;

    let messages = application::conversation_service::list_messages(pool, conversation_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // One learner + one graph handle for the whole history: the matcher
    // itself is per-message but bounded, and local SQLite keeps this cheap.
    // A missing learner (fresh DB) simply yields unannotated history.
    let graph = application::conversation_service::ensure_default_learner(pool).await.ok().map(
        |learner_id| {
            (
                learner_id,
                application::graph_service::GraphService::new(std::sync::Arc::new(pool.clone())),
            )
        },
    );

    let mut parent = NO_PARENT.to_string();
    let mut out: Vec<Value> = Vec::with_capacity(messages.len());
    for message in &messages {
        let mut value = msg_json(message, &parent, None);
        if let Some((learner_id, graph)) = &graph {
            let is_assistant = matches!(message.role, domain::MessageRole::Assistant);
            if is_assistant && !message.content.trim().is_empty() {
                match graph.annotate_turn(*learner_id, &message.content, None).await {
                    Ok(annotations) if !annotations.is_empty() => {
                        if let Value::Object(ref mut map) = value {
                            map.insert(
                                "concept_annotations".into(),
                                serde_json::to_value(&annotations).unwrap_or(Value::Null),
                            );
                        }
                    }
                    Ok(_) => {}
                    Err(e) => tracing::warn!(error = %e, "history annotation failed"),
                }
            }
        }
        out.push(value);
        parent = message.id.to_string();
    }

    Ok(Json(Value::Array(out)))
}
