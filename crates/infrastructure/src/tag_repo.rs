//! Learner conversation-tag persistence (Phase 5 bookmarks).
//!
//! Tags are learner-scoped; membership rows cascade off both sides
//! (conversation delete, tag delete). Counts are always computed live.

use chrono::{DateTime, Utc};
use domain::ConversationTag;
use sqlx::SqlitePool;
use uuid::Uuid;

fn parse_dt(s: &str) -> DateTime<Utc> {
    s.parse::<DateTime<Utc>>().unwrap_or_else(|_| Utc::now())
}

/// All tags for the learner with live conversation counts, position-ordered.
pub async fn list_tags(
    pool: &SqlitePool,
    learner_id: Uuid,
) -> Result<Vec<ConversationTag>, sqlx::Error> {
    let learner_s = learner_id.to_string();
    let rows = sqlx::query_as::<_, (String, Option<String>, i64, String, String, i64)>(
        "SELECT t.tag, t.description, t.position, t.created_at, t.updated_at, \
                (SELECT COUNT(*) FROM conversation_tag_map m \
                  WHERE m.tag = t.tag AND m.learner_id = t.learner_id) \
          FROM conversation_tags t WHERE t.learner_id = ? \
          ORDER BY t.position ASC, t.tag ASC",
    )
    .bind(&learner_s)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(tag, description, position, created, updated, count)| ConversationTag {
            tag,
            description,
            position,
            count,
            created_at: parse_dt(&created),
            updated_at: parse_dt(&updated),
        })
        .collect())
}

pub async fn get_tag(
    pool: &SqlitePool,
    learner_id: Uuid,
    tag: &str,
) -> Result<Option<ConversationTag>, sqlx::Error> {
    let learner_s = learner_id.to_string();
    let row = sqlx::query_as::<_, (String, Option<String>, i64, String, String, i64)>(
        "SELECT t.tag, t.description, t.position, t.created_at, t.updated_at, \
                (SELECT COUNT(*) FROM conversation_tag_map m \
                  WHERE m.tag = t.tag AND m.learner_id = t.learner_id) \
          FROM conversation_tags t WHERE t.learner_id = ? AND t.tag = ?",
    )
    .bind(&learner_s)
    .bind(tag)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(tag, description, position, created, updated, count)| ConversationTag {
        tag,
        description,
        position,
        count,
        created_at: parse_dt(&created),
        updated_at: parse_dt(&updated),
    }))
}

/// Creates a tag (idempotent on name: returns the existing row), appended
/// after the current maximum position.
pub async fn create_tag(
    pool: &SqlitePool,
    learner_id: Uuid,
    tag: &str,
    description: Option<&str>,
) -> Result<ConversationTag, sqlx::Error> {
    let learner_s = learner_id.to_string();
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO conversation_tags (tag, learner_id, description, position, created_at, updated_at) \
         VALUES (?, ?, ?, COALESCE((SELECT MAX(position) + 1 FROM conversation_tags WHERE learner_id = ?), 0), ?, ?) \
         ON CONFLICT (tag, learner_id) DO NOTHING",
    )
    .bind(tag)
    .bind(&learner_s)
    .bind(description)
    .bind(&learner_s)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(get_tag(pool, learner_id, tag).await?.expect("tag just created"))
}

/// Renames a tag and/or edits description/position atomically with its
/// membership rows (the map keys on the tag name).
pub async fn update_tag(
    pool: &SqlitePool,
    learner_id: Uuid,
    old_tag: &str,
    new_tag: Option<&str>,
    description: Option<Option<&str>>,
    position: Option<i64>,
) -> Result<Option<ConversationTag>, sqlx::Error> {
    let learner_s = learner_id.to_string();
    let now = Utc::now().to_rfc3339();
    let mut tx = pool.begin().await?;

    let existing: Option<(String,)> =
        sqlx::query_as("SELECT tag FROM conversation_tags WHERE learner_id = ? AND tag = ?")
            .bind(&learner_s)
            .bind(old_tag)
            .fetch_optional(&mut *tx)
            .await?;
    if existing.is_none() {
        return Ok(None);
    }
    let final_tag = new_tag.unwrap_or(old_tag);

    if final_tag != old_tag {
        sqlx::query(
            "UPDATE conversation_tags SET tag = ?, updated_at = ? WHERE learner_id = ? AND tag = ?",
        )
        .bind(final_tag)
        .bind(&now)
        .bind(&learner_s)
        .bind(old_tag)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE conversation_tag_map SET tag = ? WHERE learner_id = ? AND tag = ?")
            .bind(final_tag)
            .bind(&learner_s)
            .bind(old_tag)
            .execute(&mut *tx)
            .await?;
    }
    if let Some(description) = description {
        sqlx::query("UPDATE conversation_tags SET description = ?, updated_at = ? WHERE learner_id = ? AND tag = ?")
            .bind(description)
            .bind(&now)
            .bind(&learner_s)
            .bind(final_tag)
            .execute(&mut *tx)
            .await?;
    }
    if let Some(position) = position {
        sqlx::query("UPDATE conversation_tags SET position = ?, updated_at = ? WHERE learner_id = ? AND tag = ?")
            .bind(position)
            .bind(&now)
            .bind(&learner_s)
            .bind(final_tag)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;

    get_tag(pool, learner_id, final_tag).await
}

/// Deletes a tag and all its memberships. Returns `false` when missing.
pub async fn delete_tag(
    pool: &SqlitePool,
    learner_id: Uuid,
    tag: &str,
) -> Result<bool, sqlx::Error> {
    let learner_s = learner_id.to_string();
    let result = sqlx::query("DELETE FROM conversation_tags WHERE learner_id = ? AND tag = ?")
        .bind(&learner_s)
        .bind(tag)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Replaces a conversation's tag set (creating unknown tags on the fly so
/// create-then-attach races stay idempotent). Returns `false` when the
/// conversation does not belong to the learner.
pub async fn set_conversation_tags(
    pool: &SqlitePool,
    learner_id: Uuid,
    conversation_id: Uuid,
    tags: &[String],
) -> Result<Option<Vec<String>>, sqlx::Error> {
    let learner_s = learner_id.to_string();
    let convo_s = conversation_id.to_string();

    let mut tx = pool.begin().await?;
    let exists: Option<(String,)> =
        sqlx::query_as("SELECT id FROM conversations WHERE id = ? AND learner_id = ?")
            .bind(&convo_s)
            .bind(&learner_s)
            .fetch_optional(&mut *tx)
            .await?;
    if exists.is_none() {
        return Ok(None);
    }
    for tag in tags {
        sqlx::query(
            "INSERT INTO conversation_tags (tag, learner_id, position) \
             VALUES (?, ?, COALESCE((SELECT MAX(position) + 1 FROM conversation_tags WHERE learner_id = ?), 0)) \
             ON CONFLICT (tag, learner_id) DO NOTHING",
        )
        .bind(tag)
        .bind(&learner_s)
        .bind(&learner_s)
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query("DELETE FROM conversation_tag_map WHERE conversation_id = ? AND learner_id = ?")
        .bind(&convo_s)
        .bind(&learner_s)
        .execute(&mut *tx)
        .await?;
    for tag in tags {
        sqlx::query(
            "INSERT INTO conversation_tag_map (conversation_id, tag, learner_id) VALUES (?, ?, ?)",
        )
        .bind(&convo_s)
        .bind(tag)
        .bind(&learner_s)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    tags_for_conversation(pool, learner_id, conversation_id).await.map(Some)
}

/// A conversation's tags, name-ordered for stable display.
pub async fn tags_for_conversation(
    pool: &SqlitePool,
    learner_id: Uuid,
    conversation_id: Uuid,
) -> Result<Vec<String>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String,)>(
        "SELECT tag FROM conversation_tag_map WHERE conversation_id = ? AND learner_id = ? ORDER BY tag ASC",
    )
    .bind(conversation_id.to_string())
    .bind(learner_id.to_string())
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|(tag,)| tag).collect())
}

/// Tags for many conversations in one round-trip: `(conversation_id, tag)`
/// pairs, name-ordered per conversation.
pub async fn tags_for_conversations(
    pool: &SqlitePool,
    learner_id: Uuid,
    conversation_ids: &[Uuid],
) -> Result<Vec<(Uuid, String)>, sqlx::Error> {
    if conversation_ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = vec!["?"; conversation_ids.len()].join(",");
    let sql = format!(
        "SELECT conversation_id, tag FROM conversation_tag_map \
         WHERE learner_id = ? AND conversation_id IN ({placeholders}) ORDER BY tag ASC"
    );
    let mut q = sqlx::query_as::<_, (String, String)>(&sql);
    q = q.bind(learner_id.to_string());
    for id in conversation_ids {
        q = q.bind(id.to_string());
    }
    let rows = q.fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .filter_map(|(id, tag)| id.parse::<Uuid>().ok().map(|id| (id, tag)))
        .collect())
}
