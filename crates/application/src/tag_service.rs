//! Conversation-tag orchestration (Phase 5 bookmarks).
//!
//! Thin validation over [`infrastructure::tag_repo`]: tag names arrive
//! trimmed and non-empty; the repository owns ordering, counts, and
//! transactional membership.

use domain::ConversationTag;
use sqlx::SqlitePool;
use uuid::Uuid;

pub const MAX_TAG_LEN: usize = 100;

#[derive(Debug, thiserror::Error)]
pub enum TagError {
    #[error("tag name must not be empty")]
    EmptyName,
    #[error("tag name too long (max {MAX_TAG_LEN} chars)")]
    NameTooLong,
    #[error("tag already exists: {0}")]
    AlreadyExists(String),
    #[error("tag not found: {0}")]
    NotFound(String),
    #[error("conversation not found")]
    ConversationNotFound,
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
}

fn clean_name(raw: &str) -> Result<String, TagError> {
    let name = raw.trim().to_string();
    if name.is_empty() {
        return Err(TagError::EmptyName);
    }
    if name.chars().count() > MAX_TAG_LEN {
        return Err(TagError::NameTooLong);
    }
    Ok(name)
}

pub async fn list_tags(
    pool: &SqlitePool,
    learner_id: Uuid,
) -> Result<Vec<ConversationTag>, TagError> {
    Ok(infrastructure::tag_repo::list_tags(pool, learner_id).await?)
}

pub async fn create_tag(
    pool: &SqlitePool,
    learner_id: Uuid,
    raw_tag: &str,
    description: Option<&str>,
) -> Result<ConversationTag, TagError> {
    let tag = clean_name(raw_tag)?;
    if infrastructure::tag_repo::get_tag(pool, learner_id, &tag).await?.is_some() {
        return Err(TagError::AlreadyExists(tag));
    }
    let description = description.map(str::trim).filter(|d| !d.is_empty());
    Ok(infrastructure::tag_repo::create_tag(pool, learner_id, &tag, description).await?)
}

pub async fn rename_tag(
    pool: &SqlitePool,
    learner_id: Uuid,
    raw_old: &str,
    raw_new: Option<&str>,
    description: Option<Option<&str>>,
    position: Option<i64>,
) -> Result<ConversationTag, TagError> {
    let old = clean_name(raw_old)?;
    let new = raw_new.map(clean_name).transpose()?.unwrap_or_else(|| old.clone());
    if new != old && infrastructure::tag_repo::get_tag(pool, learner_id, &new).await?.is_some() {
        return Err(TagError::AlreadyExists(new));
    }
    infrastructure::tag_repo::update_tag(pool, learner_id, &old, Some(&new), description, position)
        .await?
        .ok_or(TagError::NotFound(old))
}

/// Sets explicit position (panel drag-reorder persistence).
pub async fn reposition_tag(
    pool: &SqlitePool,
    learner_id: Uuid,
    raw_tag: &str,
    position: i64,
) -> Result<ConversationTag, TagError> {
    let tag = clean_name(raw_tag)?;
    infrastructure::tag_repo::update_tag(pool, learner_id, &tag, None, None, Some(position))
        .await?
        .ok_or(TagError::NotFound(tag))
}

pub async fn delete_tag(
    pool: &SqlitePool,
    learner_id: Uuid,
    raw_tag: &str,
) -> Result<ConversationTag, TagError> {
    let tag = clean_name(raw_tag)?;
    let existing = infrastructure::tag_repo::get_tag(pool, learner_id, &tag)
        .await?
        .ok_or_else(|| TagError::NotFound(tag.clone()))?;
    infrastructure::tag_repo::delete_tag(pool, learner_id, &tag).await?;
    Ok(existing)
}

pub async fn set_conversation_tags(
    pool: &SqlitePool,
    learner_id: Uuid,
    conversation_id: Uuid,
    raw_tags: &[String],
) -> Result<Vec<String>, TagError> {
    let mut tags = Vec::with_capacity(raw_tags.len());
    for raw in raw_tags {
        tags.push(clean_name(raw)?);
    }
    infrastructure::tag_repo::set_conversation_tags(pool, learner_id, conversation_id, &tags)
        .await?
        .ok_or(TagError::ConversationNotFound)
}

pub async fn tags_for_conversation(
    pool: &SqlitePool,
    learner_id: Uuid,
    conversation_id: Uuid,
) -> Result<Vec<String>, TagError> {
    Ok(infrastructure::tag_repo::tags_for_conversation(pool, learner_id, conversation_id).await?)
}

pub async fn tags_for_conversations(
    pool: &SqlitePool,
    learner_id: Uuid,
    conversation_ids: &[Uuid],
) -> Result<Vec<(Uuid, String)>, TagError> {
    Ok(infrastructure::tag_repo::tags_for_conversations(pool, learner_id, conversation_ids).await?)
}
