//! Public share-link persistence (Phase 5 minimal share, F27).

use chrono::{DateTime, Utc};
use domain::SharedLink;
use sqlx::SqlitePool;
use uuid::Uuid;

fn parse_dt(s: &str) -> DateTime<Utc> {
    s.parse::<DateTime<Utc>>().unwrap_or_else(|_| Utc::now())
}

fn parse_opt_uuid(s: Option<String>) -> Option<Uuid> {
    s.and_then(|v| v.parse().ok())
}

fn row_to_link(
    row: (String, String, Option<String>, Option<String>, String, String),
) -> SharedLink {
    let (share_id, conversation_id, title, target_message_id, created, updated) = row;
    SharedLink {
        share_id: share_id.parse().unwrap(),
        conversation_id: conversation_id.parse().unwrap(),
        title,
        target_message_id: parse_opt_uuid(target_message_id),
        created_at: parse_dt(&created),
        updated_at: parse_dt(&updated),
    }
}

/// Creates a share link for a learner-owned conversation. Returns `None`
/// when the conversation does not belong to the learner.
pub async fn create_link(
    pool: &SqlitePool,
    learner_id: Uuid,
    conversation_id: Uuid,
    target_message_id: Option<Uuid>,
) -> Result<Option<SharedLink>, sqlx::Error> {
    let learner_s = learner_id.to_string();
    let convo_s = conversation_id.to_string();
    let mut tx = pool.begin().await?;

    let title: Option<(Option<String>,)> =
        sqlx::query_as("SELECT title FROM conversations WHERE id = ? AND learner_id = ?")
            .bind(&convo_s)
            .bind(&learner_s)
            .fetch_optional(&mut *tx)
            .await?;
    let Some((title,)) = title else {
        return Ok(None);
    };

    let share_id = Uuid::new_v4();
    let now = Utc::now();
    let now_s = now.to_rfc3339();
    sqlx::query(
        "INSERT INTO shared_links (share_id, conversation_id, learner_id, title, target_message_id, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(share_id.to_string())
    .bind(&convo_s)
    .bind(&learner_s)
    .bind(&title)
    .bind(target_message_id.map(|id| id.to_string()))
    .bind(&now_s)
    .bind(&now_s)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Some(SharedLink {
        share_id,
        conversation_id,
        title,
        target_message_id,
        created_at: now,
        updated_at: now,
    }))
}

/// The learner's link for a conversation, if any (newest wins).
pub async fn link_for_conversation(
    pool: &SqlitePool,
    learner_id: Uuid,
    conversation_id: Uuid,
) -> Result<Option<SharedLink>, sqlx::Error> {
    let row =
        sqlx::query_as::<_, (String, String, Option<String>, Option<String>, String, String)>(
            "SELECT share_id, conversation_id, title, target_message_id, created_at, updated_at \
         FROM shared_links WHERE learner_id = ? AND conversation_id = ? \
         ORDER BY created_at DESC LIMIT 1",
        )
        .bind(learner_id.to_string())
        .bind(conversation_id.to_string())
        .fetch_optional(pool)
        .await?;
    Ok(row.map(row_to_link))
}

/// Resolves a public share id (no learner scoping: the id is the capability).
pub async fn get_link(
    pool: &SqlitePool,
    share_id: Uuid,
) -> Result<Option<SharedLink>, sqlx::Error> {
    let row =
        sqlx::query_as::<_, (String, String, Option<String>, Option<String>, String, String)>(
            "SELECT share_id, conversation_id, title, target_message_id, created_at, updated_at \
         FROM shared_links WHERE share_id = ?",
        )
        .bind(share_id.to_string())
        .fetch_optional(pool)
        .await?;
    Ok(row.map(row_to_link))
}

/// Retargets a learner-owned link. Returns `false` when missing.
pub async fn retarget_link(
    pool: &SqlitePool,
    learner_id: Uuid,
    share_id: Uuid,
    target_message_id: Option<Uuid>,
) -> Result<Option<SharedLink>, sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE shared_links SET target_message_id = ?, updated_at = ? WHERE share_id = ? AND learner_id = ?",
    )
    .bind(target_message_id.map(|id| id.to_string()))
    .bind(&now)
    .bind(share_id.to_string())
    .bind(learner_id.to_string())
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Ok(None);
    }
    get_link(pool, share_id).await
}

/// Revokes a learner-owned link. Returns `false` when missing.
pub async fn delete_link(
    pool: &SqlitePool,
    learner_id: Uuid,
    share_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM shared_links WHERE share_id = ? AND learner_id = ?")
        .bind(share_id.to_string())
        .bind(learner_id.to_string())
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
