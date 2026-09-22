//! Conversation and message endpoints for the LibreChat client.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use super::{conv_json_tags, derive_title, msg_json, NO_PARENT};
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
/// `isArchived`/`pinned` filters serve the archive view and the pinned
/// section (Phase 5 pin/archive); other params are accepted and ignored.
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(rename = "isArchived")]
    pub is_archived: Option<bool>,
    pub pinned: Option<bool>,
}

pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Value>, AppError> {
    let pool = pool(&state).await?;
    let convs = application::conversation_service::list_conversations_filtered(
        pool,
        query.is_archived,
        query.pinned,
    )
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    let conversations: Vec<Value> = {
        let learner_id = application::conversation_service::default_learner_id();
        let ids: Vec<Uuid> = convs.iter().map(|c| c.id).collect();
        let pairs = application::tag_service::tags_for_conversations(pool, learner_id, &ids)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let mut by_convo: std::collections::HashMap<Uuid, Vec<String>> =
            std::collections::HashMap::new();
        for (id, tag) in pairs {
            by_convo.entry(id).or_default().push(tag);
        }
        convs
            .iter()
            .map(|c| {
                let empty = Vec::new();
                let tags = by_convo.get(&c.id).unwrap_or(&empty);
                conv_json_tags(c, tags)
            })
            .collect()
    };
    Ok(Json(json!({
        "conversations": conversations,
        "nextCursor": Value::Null,
    })))
}

/// `GET /api/messages?search=q&pageSize=n&cursor=o` — global message
/// search for the `/search` page (Phase 5 conversation search).
///
/// Bounded SQLite substring match (no MeiliSearch daemon — the scale does
/// not warrant one); blank queries return an empty page. `cursor` is an
/// opaque offset: absent or non-empty with remaining hits yields the next
/// one, otherwise `null` terminates pagination like upstream.
#[derive(Debug, Deserialize)]
pub struct MessageSearchQuery {
    pub search: Option<String>,
    #[serde(rename = "pageSize")]
    pub page_size: Option<i64>,
    pub cursor: Option<String>,
}

pub async fn search_messages(
    State(state): State<AppState>,
    Query(params): Query<MessageSearchQuery>,
) -> Result<Json<Value>, AppError> {
    let pool = pool(&state).await?;
    let query = params.search.as_deref().unwrap_or("").trim();
    if query.is_empty() {
        return Ok(Json(json!({ "messages": [], "nextCursor": Value::Null })));
    }
    let limit = params.page_size.unwrap_or(20).clamp(1, 100);
    let offset: i64 = params.cursor.as_deref().unwrap_or("0").parse().unwrap_or(0).max(0);
    let learner_id = application::conversation_service::default_learner_id();
    let (messages, total) =
        infrastructure::conversation_repo::search_messages(pool, learner_id, query, limit, offset)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
    let messages: Vec<Value> = messages
        .iter()
        .map(|(m, title)| {
            let mut v = msg_json(m, NO_PARENT, None);
            if let Some(obj) = v.as_object_mut() {
                // Search rows navigate by conversation and display its real
                // title — never the history placeholder.
                obj.insert(
                    "title".to_string(),
                    Value::String(
                        title
                            .clone()
                            .filter(|t| !t.trim().is_empty())
                            .unwrap_or_else(|| "New Chat".to_string()),
                    ),
                );
            }
            v
        })
        .collect();
    let next = offset + limit;
    let next_cursor = if next < total { Value::String(next.to_string()) } else { Value::Null };
    Ok(Json(json!({ "messages": messages, "nextCursor": next_cursor })))
}

/// `GET /api/messages/:conversation_id/:message_id` — single message fetch
/// (Phase 5 share flow: target resolution). Shape matches the client's
/// `getMessageById` (array with the one message).
pub async fn get_message(
    State(state): State<AppState>,
    Path((conversation_id, message_id)): Path<(String, String)>,
) -> Result<Json<Value>, AppError> {
    let pool = pool(&state).await?;
    let conversation_id = parse_uuid(&conversation_id)?;
    let message_id = parse_uuid(&message_id)?;
    let message = infrastructure::conversation_repo::get_message(pool, conversation_id, message_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("message not found".into()))?;
    Ok(Json(json!([msg_json(&message, NO_PARENT, None)])))
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
    let learner_id = application::conversation_service::default_learner_id();
    let tags = application::tag_service::tags_for_conversation(pool, learner_id, conversation_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(conv_json_tags(&conv, &tags)))
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
    let learner_id = application::conversation_service::default_learner_id();
    let tags = application::tag_service::tags_for_conversation(pool, learner_id, conversation_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(conv_json_tags(&updated, &tags)))
}

#[derive(Deserialize)]
pub struct ArchiveRequest {
    pub arg: ArchiveArg,
}

#[derive(Deserialize)]
pub struct ArchiveArg {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    #[serde(rename = "isArchived")]
    pub is_archived: bool,
}

/// `POST /api/convos/archive` — archives or unarchives one conversation.
pub async fn archive(
    State(state): State<AppState>,
    Json(body): Json<ArchiveRequest>,
) -> Result<Json<Value>, AppError> {
    let pool = pool(&state).await?;
    let conversation_id = parse_uuid(&body.arg.conversation_id)?;
    let updated = application::conversation_service::set_archived(
        pool,
        conversation_id,
        body.arg.is_archived,
    )
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;
    if !updated {
        return Err(AppError::NotFound("conversation not found".into()));
    }
    let learner_id = application::conversation_service::default_learner_id();
    let conv = application::conversation_service::get_conversation(pool, conversation_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("conversation not found".into()))?;
    let tags = application::tag_service::tags_for_conversation(pool, learner_id, conversation_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(conv_json_tags(&conv, &tags)))
}

/// `POST /api/convos/archive/all` — archives every unarchived conversation.
pub async fn archive_all(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let pool = pool(&state).await?;
    let count = application::conversation_service::archive_all(pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(json!({ "archivedCount": count })))
}

#[derive(Deserialize)]
pub struct PinRequest {
    pub arg: PinArg,
}

#[derive(Deserialize)]
pub struct PinArg {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    pub pinned: bool,
}

/// `POST /api/convos/pin` — pins or unpins one conversation.
pub async fn pin(
    State(state): State<AppState>,
    Json(body): Json<PinRequest>,
) -> Result<Json<Value>, AppError> {
    let pool = pool(&state).await?;
    let conversation_id = parse_uuid(&body.arg.conversation_id)?;
    let updated =
        application::conversation_service::set_pinned(pool, conversation_id, body.arg.pinned)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
    if !updated {
        return Err(AppError::NotFound("conversation not found".into()));
    }
    let learner_id = application::conversation_service::default_learner_id();
    let conv = application::conversation_service::get_conversation(pool, conversation_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("conversation not found".into()))?;
    let tags = application::tag_service::tags_for_conversation(pool, learner_id, conversation_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(conv_json_tags(&conv, &tags)))
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

#[derive(Deserialize)]
pub struct DuplicateRequest {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
}

/// `POST /api/convos/duplicate` — copies a conversation with its messages,
/// flags, and tag membership under a fresh id (`title + " (copy)"`).
pub async fn duplicate(
    State(state): State<AppState>,
    Json(body): Json<DuplicateRequest>,
) -> Result<Json<Value>, AppError> {
    let pool = pool(&state).await?;
    let conversation_id = parse_uuid(&body.conversation_id)?;
    let (conv, messages) =
        application::conversation_service::duplicate_conversation(pool, conversation_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("conversation not found".into()))?;
    let learner_id = application::conversation_service::default_learner_id();
    let tags = application::tag_service::tags_for_conversation(pool, learner_id, conv.id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let mut parent = NO_PARENT.to_string();
    let mut out = Vec::with_capacity(messages.len());
    for message in &messages {
        out.push(msg_json(message, &parent, None));
        parent = message.id.to_string();
    }
    Ok(Json(json!({ "conversation": conv_json_tags(&conv, &tags), "messages": out })))
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
