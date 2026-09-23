//! Minimal conversation sharing (Phase 5, F27) — the deliberate narrow
//! exception to no-collaboration: a learner may share a useful
//! explanation. Nothing here grows into profiles, comments, or feeds.
//!
//! Served contract (everything the bundled flows call):
//! ```text
//! POST   /api/share/:conversationId   { targetMessageId? }  (owner: create)
//! GET    /api/share/link/:conversationId                    (owner: existing link)
//! PATCH  /api/share/:shareId          { targetMessageId? }  (owner: retarget)
//! DELETE /api/share/:shareId                                (owner: revoke)
//! GET    /api/share/:shareId                                (public: messages)
//! GET    /api/share/:shareId/config                         (public: render config)
//! POST   /api/share/:shareId/fork     { targetMessageIndex? } (viewer: own copy)
//! ```
//! Deliberately unserved (JSON 404): the shared-links LIST page, shared
//! files (no files feature exists to snapshot), and expiry (revocation is
//! explicit delete). Share ids are unguessable UUIDs unrelated to
//! conversation ids; that unguessability IS the read capability.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{conv_json_tags, msg_json, NO_PARENT};
use crate::error::AppError;
use crate::routes::AppState;

async fn pool(state: &AppState) -> Result<&sqlx::SqlitePool, AppError> {
    state.pool.as_ref().ok_or_else(|| AppError::Internal("no db pool".into()))
}

fn parse_uuid(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|_| AppError::Validation(format!("invalid id: {raw}")))
}

fn parse_opt_uuid(raw: Option<&str>) -> Result<Option<Uuid>, AppError> {
    raw.map(parse_uuid).transpose()
}

#[derive(Debug, Serialize)]
pub struct ShareLinkJson {
    #[serde(rename = "_id")]
    id: String,
    #[serde(rename = "shareId")]
    share_id: String,
    #[serde(rename = "targetMessageId")]
    target_message_id: Option<String>,
    #[serde(rename = "conversationId")]
    conversation_id: String,
    title: Option<String>,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
}

fn link_json(link: domain::SharedLink) -> ShareLinkJson {
    ShareLinkJson {
        id: link.share_id.to_string(),
        share_id: link.share_id.to_string(),
        target_message_id: link.target_message_id.map(|id| id.to_string()),
        conversation_id: link.conversation_id.to_string(),
        title: link.title,
        created_at: link.created_at.to_rfc3339(),
        updated_at: link.updated_at.to_rfc3339(),
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateLinkBody {
    #[serde(rename = "targetMessageId")]
    pub target_message_id: Option<String>,
}

/// `POST /api/share/:conversationId` — creates a share link (always a new
/// unguessable id, even if one exists; the owner reads the current one via
/// the link endpoint).
pub async fn create(
    State(state): State<AppState>,
    Path(conversation_id): Path<String>,
    Json(body): Json<CreateLinkBody>,
) -> Result<Json<ShareLinkJson>, AppError> {
    let pool = pool(&state).await?;
    let conversation_id = parse_uuid(&conversation_id)?;
    let target = parse_opt_uuid(body.target_message_id.as_deref())?;
    let link = application::share_service::create_link(pool, conversation_id, target)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("conversation not found".into()))?;
    Ok(Json(link_json(link)))
}

/// `GET /api/share/link/:conversationId` — the owner's current link, if any.
pub async fn get_for_conversation(
    State(state): State<AppState>,
    Path(conversation_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = pool(&state).await?;
    let conversation_id = parse_uuid(&conversation_id)?;
    let link = application::share_service::link_for_conversation(pool, conversation_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    match link {
        Some(link) => Ok(Json(serde_json::json!({
            "shareId": link.share_id.to_string(),
            "success": true,
            "conversationId": conversation_id.to_string(),
        }))),
        None => Ok(Json(serde_json::json!({
            "shareId": serde_json::Value::Null,
            "success": true,
            "conversationId": conversation_id.to_string(),
        }))),
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateLinkBody {
    #[serde(rename = "targetMessageId")]
    pub target_message_id: Option<String>,
}

/// `PATCH /api/share/:shareId` — retargets a link (message scope).
pub async fn update(
    State(state): State<AppState>,
    Path(share_id): Path<String>,
    Json(body): Json<UpdateLinkBody>,
) -> Result<Json<ShareLinkJson>, AppError> {
    let pool = pool(&state).await?;
    let share_id = parse_uuid(&share_id)?;
    let target = parse_opt_uuid(body.target_message_id.as_deref())?;
    let link = application::share_service::retarget_link(pool, share_id, target)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("share link not found".into()))?;
    Ok(Json(link_json(link)))
}

/// `DELETE /api/share/:shareId` — revokes a link immediately.
pub async fn delete(
    State(state): State<AppState>,
    Path(share_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = pool(&state).await?;
    let share_id = parse_uuid(&share_id)?;
    let removed = application::share_service::delete_link(pool, share_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    if !removed {
        return Err(AppError::NotFound("share link not found".into()));
    }
    Ok(Json(serde_json::json!({ "success": true })))
}

/// `GET /api/share/:shareId` — public message payload (no auth: the id is
/// the capability). Honors the link's target scope.
pub async fn read(
    State(state): State<AppState>,
    Path(share_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = pool(&state).await?;
    let share_id = parse_uuid(&share_id)?;
    let link = application::share_service::get_link(pool, share_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("share link not found".into()))?;
    let messages = application::share_service::link_messages(pool, &link)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let mut out = Vec::with_capacity(messages.len());
    for message in &messages {
        let parent_str = message
            .parent_message_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| NO_PARENT.to_string());
        out.push(msg_json(message, &parent_str, None));
    }
    Ok(Json(serde_json::json!({
        "shareId": link.share_id.to_string(),
        "conversationId": link.conversation_id.to_string(),
        "title": link.title,
        "messages": out,
        "createdAt": link.created_at.to_rfc3339(),
        "updatedAt": link.updated_at.to_rfc3339(),
    })))
}

/// `GET /api/share/:shareId/config` — minimal render config for the public
/// share view (app title only; no provider or interface surface).
pub async fn render_config(
    State(state): State<AppState>,
    Path(share_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = pool(&state).await?;
    let share_id = parse_uuid(&share_id)?;
    application::share_service::get_link(pool, share_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("share link not found".into()))?;
    Ok(Json(serde_json::json!({ "appTitle": "Knowledgeable" })))
}

#[derive(Debug, Deserialize)]
pub struct ForkBody {
    #[serde(rename = "targetMessageIndex")]
    pub target_message_index: Option<usize>,
    #[serde(rename = "shareRevision")]
    pub _share_revision: Option<String>,
}

/// `POST /api/share/:shareId/fork` — copies the (possibly scoped) shared
/// conversation into the viewer's own history (single-user: the learner).
pub async fn fork(
    State(state): State<AppState>,
    Path(share_id): Path<String>,
    Json(body): Json<ForkBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = pool(&state).await?;
    let share_id = parse_uuid(&share_id)?;
    let link = application::share_service::get_link(pool, share_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("share link not found".into()))?;
    let (conv, messages) =
        application::share_service::fork_link(pool, &link, body.target_message_index)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("shared conversation not found".into()))?;
    let learner_id = application::conversation_service::default_learner_id();
    let tags = application::tag_service::tags_for_conversation(pool, learner_id, conv.id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let mut out = Vec::with_capacity(messages.len());
    for message in &messages {
        let parent_str = message
            .parent_message_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| NO_PARENT.to_string());
        out.push(msg_json(message, &parent_str, None));
    }
    Ok(Json(serde_json::json!({
        "conversation": conv_json_tags(&conv, &tags),
        "messages": out,
    })))
}
