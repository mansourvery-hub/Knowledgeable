//! Conversation-tag endpoints for bookmarks (Phase 5, F19).
//!
//! Thin controller over [`application::tag_service`]. Shapes mirror the
//! upstream `TConversationTag` the client parses (`_id`/`tag`/`user`/
//! `description`/`count`/`position`/`createdAt`/`updatedAt`); single-user
//! `user` is the default learner id.
//!
//! Served contract (everything the bundled client calls):
//! ```text
//! GET    /api/tags
//! POST   /api/tags                        { tag, description?, conversationId?, addToConversation? }
//! PUT    /api/tags/:tag                   { tag?, description?, position? }
//! DELETE /api/tags/:tag
//! PUT    /api/tags/convo/:conversationId  { tags: string[], tag }
//! ```
//! Deliberately unserved (JSON 404 like every other dormant surface):
//! `/api/tags/list` pagination and `/api/tags/rebuild` — nothing in the
//! bundled flows calls them.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;
use crate::routes::AppState;

async fn pool(state: &AppState) -> Result<&sqlx::SqlitePool, AppError> {
    state.pool.as_ref().ok_or_else(|| AppError::Internal("no db pool".into()))
}

async fn learner(pool: &sqlx::SqlitePool) -> Result<Uuid, AppError> {
    application::conversation_service::ensure_default_learner(pool)
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("database error: {e}")))
}

fn parse_uuid(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|_| AppError::Validation(format!("invalid id: {raw}")))
}

#[derive(Debug, Serialize)]
pub struct TagJson {
    #[serde(rename = "_id")]
    id: String,
    tag: String,
    user: String,
    description: Option<String>,
    count: i64,
    position: i64,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
}

fn tag_json(learner_id: Uuid, tag: domain::ConversationTag) -> TagJson {
    TagJson {
        id: tag.tag.clone(),
        tag: tag.tag,
        user: learner_id.to_string(),
        description: tag.description,
        count: tag.count,
        position: tag.position,
        created_at: tag.created_at.to_rfc3339(),
        updated_at: tag.updated_at.to_rfc3339(),
    }
}

fn tag_error(e: application::tag_service::TagError) -> AppError {
    use application::tag_service::TagError as E;
    match e {
        E::EmptyName | E::NameTooLong => AppError::Validation(e.to_string()),
        E::AlreadyExists(name) => AppError::Conflict(format!("tag already exists: {name}")),
        E::NotFound(name) => AppError::NotFound(format!("tag not found: {name}")),
        E::ConversationNotFound => AppError::NotFound("conversation not found".into()),
        E::Db(inner) => AppError::ServiceUnavailable(format!("database error: {inner}")),
    }
}

/// `GET /api/tags` — all learner tags with live counts, position-ordered.
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<TagJson>>, AppError> {
    let pool = pool(&state).await?;
    let learner_id = learner(pool).await?;
    let tags = application::tag_service::list_tags(pool, learner_id).await.map_err(tag_error)?;
    Ok(Json(tags.into_iter().map(|t| tag_json(learner_id, t)).collect()))
}

#[derive(Debug, Deserialize)]
pub struct CreateTagBody {
    pub tag: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "conversationId")]
    pub conversation_id: Option<String>,
    #[serde(rename = "addToConversation")]
    pub add_to_conversation: Option<bool>,
}

/// `POST /api/tags` — creates a tag, optionally attaching it to a
/// conversation (union with its existing tags, never a replace).
pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<CreateTagBody>,
) -> Result<Json<TagJson>, AppError> {
    let pool = pool(&state).await?;
    let learner_id = learner(pool).await?;
    let raw = body.tag.as_deref().unwrap_or("");
    let created =
        application::tag_service::create_tag(pool, learner_id, raw, body.description.as_deref())
            .await
            .map_err(tag_error)?;
    if body.add_to_conversation == Some(true) {
        if let Some(convo_raw) = body.conversation_id.as_deref() {
            let conversation_id = parse_uuid(convo_raw)?;
            let mut current =
                application::tag_service::tags_for_conversation(pool, learner_id, conversation_id)
                    .await
                    .map_err(tag_error)?;
            if !current.iter().any(|t| t == &created.tag) {
                current.push(created.tag.clone());
            }
            application::tag_service::set_conversation_tags(
                pool,
                learner_id,
                conversation_id,
                &current,
            )
            .await
            .map_err(tag_error)?;
        }
    }
    Ok(Json(tag_json(learner_id, created)))
}

#[derive(Debug, Deserialize)]
pub struct UpdateTagBody {
    pub tag: Option<String>,
    /// Absent means "leave it"; explicit null clears it.
    #[serde(default)]
    pub description: Option<Option<String>>,
    pub position: Option<i64>,
}

/// `PUT /api/tags/:tag` — renames and/or edits description/position.
pub async fn update(
    State(state): State<AppState>,
    Path(tag): Path<String>,
    Json(body): Json<UpdateTagBody>,
) -> Result<Json<TagJson>, AppError> {
    let pool = pool(&state).await?;
    let learner_id = learner(pool).await?;
    let description: Option<Option<&str>> = body.description.as_ref().map(|inner| inner.as_deref());
    let updated = application::tag_service::rename_tag(
        pool,
        learner_id,
        &tag,
        body.tag.as_deref(),
        description,
        body.position,
    )
    .await
    .map_err(tag_error)?;
    Ok(Json(tag_json(learner_id, updated)))
}

/// `DELETE /api/tags/:tag` — deletes the tag and all its memberships.
pub async fn delete(
    State(state): State<AppState>,
    Path(tag): Path<String>,
) -> Result<Json<TagJson>, AppError> {
    let pool = pool(&state).await?;
    let learner_id = learner(pool).await?;
    let deleted =
        application::tag_service::delete_tag(pool, learner_id, &tag).await.map_err(tag_error)?;
    Ok(Json(tag_json(learner_id, deleted)))
}

#[derive(Debug, Deserialize)]
pub struct SetConvoTagsBody {
    pub tags: Option<Vec<String>>,
    /// The toggled tag (informational; the list is authoritative).
    #[allow(dead_code)]
    pub tag: Option<String>,
}

/// `PUT /api/tags/convo/:conversationId` — replaces the conversation's tag
/// set (creating unknown tags on the fly) and returns the new list.
pub async fn set_for_conversation(
    State(state): State<AppState>,
    Path(conversation_id): Path<String>,
    Json(body): Json<SetConvoTagsBody>,
) -> Result<Json<Vec<String>>, AppError> {
    let pool = pool(&state).await?;
    let learner_id = learner(pool).await?;
    let conversation_id = parse_uuid(&conversation_id)?;
    let tags = body.tags.as_deref().unwrap_or(&[]);
    let updated =
        application::tag_service::set_conversation_tags(pool, learner_id, conversation_id, tags)
            .await
            .map_err(tag_error)?;
    Ok(Json(updated))
}
