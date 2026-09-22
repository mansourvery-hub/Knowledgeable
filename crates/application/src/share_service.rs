//! Public share-link orchestration (Phase 5 minimal share, F27).
//!
//! Thin layer over [`infrastructure::share_repo`]: learner-owned links are
//! created/retargeted/revoked; reads and forks resolve the unguessable
//! share id with no learner scoping (the id is the capability).

use domain::{Conversation, ConversationMessage, SharedLink};
use sqlx::SqlitePool;
use uuid::Uuid;

async fn learner(pool: &SqlitePool) -> Result<Uuid, sqlx::Error> {
    crate::conversation_service::ensure_default_learner(pool).await
}

pub async fn create_link(
    pool: &SqlitePool,
    conversation_id: Uuid,
    target_message_id: Option<Uuid>,
) -> Result<Option<SharedLink>, sqlx::Error> {
    let learner_id = learner(pool).await?;
    infrastructure::share_repo::create_link(pool, learner_id, conversation_id, target_message_id)
        .await
}

pub async fn link_for_conversation(
    pool: &SqlitePool,
    conversation_id: Uuid,
) -> Result<Option<SharedLink>, sqlx::Error> {
    let learner_id = learner(pool).await?;
    infrastructure::share_repo::link_for_conversation(pool, learner_id, conversation_id).await
}

pub async fn retarget_link(
    pool: &SqlitePool,
    share_id: Uuid,
    target_message_id: Option<Uuid>,
) -> Result<Option<SharedLink>, sqlx::Error> {
    // Learner ownership is enforced inside the scoped update; a foreign or
    // missing id simply yields None (no oracle distinguishing the two).
    let learner_id = learner(pool).await?;
    infrastructure::share_repo::retarget_link(pool, learner_id, share_id, target_message_id).await
}

pub async fn delete_link(pool: &SqlitePool, share_id: Uuid) -> Result<bool, sqlx::Error> {
    let learner_id = learner(pool).await?;
    infrastructure::share_repo::delete_link(pool, learner_id, share_id).await
}

pub async fn get_link(
    pool: &SqlitePool,
    share_id: Uuid,
) -> Result<Option<SharedLink>, sqlx::Error> {
    infrastructure::share_repo::get_link(pool, share_id).await
}

/// Messages visible through a link, chronological, honoring an optional
/// target scope (share up to and including that message).
pub async fn link_messages(
    pool: &SqlitePool,
    link: &SharedLink,
) -> Result<Vec<ConversationMessage>, sqlx::Error> {
    // list_messages is chronological already (ORDER BY created_at ASC).
    let mut messages =
        infrastructure::conversation_repo::list_messages(pool, link.conversation_id).await?;
    if let Some(target) = link.target_message_id {
        if let Some(pos) = messages.iter().position(|m| m.id == target) {
            messages.truncate(pos + 1);
        }
    }
    Ok(messages)
}

/// Forks a shared conversation into the learner's own copy, optionally
/// capped at the viewer's active message index (0-based, inclusive,
/// intersected with the link's own target scope). Returns `None` when the
/// source conversation is gone.
pub async fn fork_link(
    pool: &SqlitePool,
    link: &SharedLink,
    target_index: Option<usize>,
) -> Result<Option<(Conversation, Vec<ConversationMessage>)>, sqlx::Error> {
    let learner_id = crate::conversation_service::default_learner_id();
    let mut max = link_messages(pool, link).await?.len();
    if let Some(i) = target_index {
        max = max.min(i.saturating_add(1));
    }
    infrastructure::conversation_repo::duplicate_conversation(
        pool,
        learner_id,
        link.conversation_id,
        Some(max),
    )
    .await
}
