use chrono::{DateTime, Utc};
use domain::{Conversation, ConversationMessage, MessageRole};
use sqlx::SqlitePool;
use uuid::Uuid;

fn parse_role(s: &str) -> MessageRole {
    s.parse().unwrap_or(MessageRole::User)
}

fn parse_dt(s: &str) -> DateTime<Utc> {
    s.parse::<DateTime<Utc>>().unwrap_or_else(|_| Utc::now())
}

pub async fn ensure_default_learner(
    pool: &SqlitePool,
    learner_id: Uuid,
) -> Result<(), sqlx::Error> {
    let id = learner_id.to_string();
    let now = Utc::now().to_rfc3339();
    sqlx::query("INSERT OR IGNORE INTO learners (id, created_at, updated_at) VALUES (?, ?, ?)")
        .bind(&id)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn create_conversation(
    pool: &SqlitePool,
    learner_id: Uuid,
    title: Option<&str>,
) -> Result<Conversation, sqlx::Error> {
    let id = Uuid::new_v4();
    let now = Utc::now();
    let now_s = now.to_rfc3339();
    let learner_s = learner_id.to_string();
    let id_s = id.to_string();

    sqlx::query(
        "INSERT INTO conversations (id, learner_id, title, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id_s)
    .bind(&learner_s)
    .bind(title)
    .bind(&now_s)
    .bind(&now_s)
    .execute(pool)
    .await?;

    Ok(Conversation {
        id,
        learner_id,
        title: title.map(|s| s.to_string()),
        is_archived: false,
        pinned: false,
        created_at: now,
        updated_at: now,
    })
}

pub async fn list_conversations(
    pool: &SqlitePool,
    learner_id: Uuid,
) -> Result<Vec<Conversation>, sqlx::Error> {
    list_conversations_filtered(pool, learner_id, None, None).await
}

/// Conversation list with optional archived/pinned filters (Phase 5
/// pin/archive). Absent `archived` reads as unarchived-only — the client
/// omits the parameter (rather than sending false) for the working
/// surface, and the archive view passes explicit `true`. Absent `pinned`
/// is unfiltered.
pub async fn list_conversations_filtered(
    pool: &SqlitePool,
    learner_id: Uuid,
    archived: Option<bool>,
    pinned: Option<bool>,
) -> Result<Vec<Conversation>, sqlx::Error> {
    let learner_s = learner_id.to_string();
    let mut sql = "SELECT id, learner_id, title, is_archived, pinned, created_at, updated_at \
                   FROM conversations WHERE learner_id = ? AND is_archived = ?"
        .to_string();
    if pinned.is_some() {
        sql.push_str(" AND pinned = ?");
    }
    sql.push_str(" ORDER BY updated_at DESC");
    let mut q =
        sqlx::query_as::<_, (String, String, Option<String>, i64, i64, String, String)>(&sql);
    q = q.bind(&learner_s);
    q = q.bind(if archived.unwrap_or(false) { 1 } else { 0 });
    if let Some(pinned) = pinned {
        q = q.bind(if pinned { 1 } else { 0 });
    }
    let rows = q.fetch_all(pool).await?;

    Ok(rows
        .into_iter()
        .map(|(id, learner_id, title, is_archived, pinned, created_at, updated_at)| Conversation {
            id: id.parse().unwrap(),
            learner_id: learner_id.parse().unwrap(),
            title,
            is_archived: is_archived != 0,
            pinned: pinned != 0,
            created_at: parse_dt(&created_at),
            updated_at: parse_dt(&updated_at),
        })
        .collect())
}

pub async fn get_conversation(
    pool: &SqlitePool,
    learner_id: Uuid,
    conversation_id: Uuid,
) -> Result<Option<Conversation>, sqlx::Error> {
    let learner_s = learner_id.to_string();
    let id_s = conversation_id.to_string();
    let row = sqlx::query_as::<_, (String, String, Option<String>, i64, i64, String, String)>(
        "SELECT id, learner_id, title, is_archived, pinned, created_at, updated_at FROM conversations WHERE id = ? AND learner_id = ?",
    )
    .bind(&id_s)
    .bind(&learner_s)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(id, learner_id, title, is_archived, pinned, created_at, updated_at)| {
        Conversation {
            id: id.parse().unwrap(),
            learner_id: learner_id.parse().unwrap(),
            title,
            is_archived: is_archived != 0,
            pinned: pinned != 0,
            created_at: parse_dt(&created_at),
            updated_at: parse_dt(&updated_at),
        }
    }))
}

pub async fn create_message(
    pool: &SqlitePool,
    conversation_id: Uuid,
    role: MessageRole,
    content: &str,
) -> Result<ConversationMessage, sqlx::Error> {
    create_message_with_id(pool, Uuid::new_v4(), conversation_id, role, content).await
}

/// Persists a message under a caller-chosen id. Used by protocol adapters that
/// must advertise a stable message id to the client before persistence completes.
pub async fn create_message_with_id(
    pool: &SqlitePool,
    id: Uuid,
    conversation_id: Uuid,
    role: MessageRole,
    content: &str,
) -> Result<ConversationMessage, sqlx::Error> {
    let now = Utc::now();
    let now_s = now.to_rfc3339();
    let id_s = id.to_string();
    let conv_s = conversation_id.to_string();
    let role_s = role.to_string();

    sqlx::query(
        "INSERT INTO conversation_messages (id, conversation_id, role, content, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id_s)
    .bind(&conv_s)
    .bind(&role_s)
    .bind(content)
    .bind(&now_s)
    .execute(pool)
    .await?;

    // bump conversation updated_at
    sqlx::query("UPDATE conversations SET updated_at = ? WHERE id = ?")
        .bind(&now_s)
        .bind(&conv_s)
        .execute(pool)
        .await?;

    Ok(ConversationMessage {
        id,
        conversation_id,
        role,
        content: content.to_string(),
        created_at: now,
    })
}

pub async fn list_messages(
    pool: &SqlitePool,
    conversation_id: Uuid,
) -> Result<Vec<ConversationMessage>, sqlx::Error> {
    let conv_s = conversation_id.to_string();
    let rows = sqlx::query_as::<_, (String, String, String, String, String)>(
        "SELECT id, conversation_id, role, content, created_at FROM conversation_messages WHERE conversation_id = ? ORDER BY created_at ASC",
    )
    .bind(&conv_s)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, conversation_id, role, content, created_at)| ConversationMessage {
            id: id.parse().unwrap(),
            conversation_id: conversation_id.parse().unwrap(),
            role: parse_role(&role),
            content,
            created_at: parse_dt(&created_at),
        })
        .collect())
}

/// Global message search for the `/search` page (Phase 5): substring
/// match over message content across the learner's conversations, newest
/// first. Returns `(message, owning conversation title)` pairs plus the
/// total hit count so callers can mint a cursor. Blank queries return
/// nothing (typeahead-friendly).
pub async fn search_messages(
    pool: &SqlitePool,
    learner_id: Uuid,
    query: &str,
    limit: i64,
    offset: i64,
) -> Result<(Vec<(ConversationMessage, Option<String>)>, i64), sqlx::Error> {
    let learner_s = learner_id.to_string();
    let like = format!("%{query}%");
    let rows = sqlx::query_as::<_, (String, String, String, String, String, Option<String>)>(
        "SELECT m.id, m.conversation_id, m.role, m.content, m.created_at, c.title \
         FROM conversation_messages m \
         JOIN conversations c ON c.id = m.conversation_id \
         WHERE c.learner_id = ? AND m.content LIKE ? \
         ORDER BY m.created_at DESC LIMIT ? OFFSET ?",
    )
    .bind(&learner_s)
    .bind(&like)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    let total: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM conversation_messages m \
         JOIN conversations c ON c.id = m.conversation_id \
         WHERE c.learner_id = ? AND m.content LIKE ?",
    )
    .bind(&learner_s)
    .bind(&like)
    .fetch_one(pool)
    .await?;

    Ok((
        rows.into_iter()
            .map(|(id, conversation_id, role, content, created_at, title)| {
                (
                    ConversationMessage {
                        id: id.parse().unwrap(),
                        conversation_id: conversation_id.parse().unwrap(),
                        role: parse_role(&role),
                        content,
                        created_at: parse_dt(&created_at),
                    },
                    title,
                )
            })
            .collect(),
        total.0,
    ))
}

/// Fetches one message by id within a conversation (Phase 5 share flow:
/// target resolution). Returns `None` when either id is unknown.
pub async fn get_message(
    pool: &SqlitePool,
    conversation_id: Uuid,
    message_id: Uuid,
) -> Result<Option<ConversationMessage>, sqlx::Error> {
    let row = sqlx::query_as::<_, (String, String, String, String, String)>(
        "SELECT id, conversation_id, role, content, created_at FROM conversation_messages WHERE id = ? AND conversation_id = ?",
    )
    .bind(message_id.to_string())
    .bind(conversation_id.to_string())
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(id, conversation_id, role, content, created_at)| ConversationMessage {
        id: id.parse().unwrap(),
        conversation_id: conversation_id.parse().unwrap(),
        role: parse_role(&role),
        content,
        created_at: parse_dt(&created_at),
    }))
}

/// Updates a conversation's title and bumps `updated_at`.
pub async fn update_conversation_title(
    pool: &SqlitePool,
    conversation_id: Uuid,
    title: &str,
) -> Result<(), sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    sqlx::query("UPDATE conversations SET title = ?, updated_at = ? WHERE id = ?")
        .bind(title)
        .bind(&now)
        .bind(conversation_id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

/// Sets the archived flag (Phase 5 pin/archive). Archiving also unpins:
/// archive files the conversation away from the working surface entirely,
/// so a pinned row never lingers in the Pinned section. Unarchiving does
/// not restore the pin. Returns `false` when the conversation does not
/// belong to the learner.
pub async fn set_archived(
    pool: &SqlitePool,
    learner_id: Uuid,
    conversation_id: Uuid,
    archived: bool,
) -> Result<bool, sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE conversations SET is_archived = ?, pinned = CASE WHEN ? = 1 THEN 0 ELSE pinned END, updated_at = ? WHERE id = ? AND learner_id = ?",
    )
    .bind(if archived { 1 } else { 0 })
    .bind(if archived { 1 } else { 0 })
    .bind(&now)
    .bind(conversation_id.to_string())
    .bind(learner_id.to_string())
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// Sets the pinned flag (Phase 5 pin/archive). Returns `false` when the
/// conversation does not belong to the learner.
pub async fn set_pinned(
    pool: &SqlitePool,
    learner_id: Uuid,
    conversation_id: Uuid,
    pinned: bool,
) -> Result<bool, sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE conversations SET pinned = ?, updated_at = ? WHERE id = ? AND learner_id = ?",
    )
    .bind(if pinned { 1 } else { 0 })
    .bind(&now)
    .bind(conversation_id.to_string())
    .bind(learner_id.to_string())
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// Archives every unarchived conversation. Returns the archived count —
/// already-archived rows are excluded so the count stays honest.
pub async fn archive_all(pool: &SqlitePool, learner_id: Uuid) -> Result<i64, sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE conversations SET is_archived = 1, updated_at = ? WHERE learner_id = ? AND is_archived = 0",
    )
    .bind(&now)
    .bind(learner_id.to_string())
    .execute(pool)
    .await?;
    Ok(result.rows_affected() as i64)
}

/// Deletes a conversation. Messages cascade via the schema's foreign key.
pub async fn delete_conversation(
    pool: &SqlitePool,
    conversation_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM conversations WHERE id = ?")
        .bind(conversation_id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

/// Duplicates a conversation with its messages, flags, and tag membership
/// (Phase 5 fork/duplicate). The copy gets fresh ids, `title + " (copy)"`,
/// and current timestamps; the source is untouched. `max_messages` caps the
/// copied prefix (fork-a-branch); `None` copies everything. Returns `None`
/// when the source does not belong to the learner.
pub async fn duplicate_conversation(
    pool: &SqlitePool,
    learner_id: Uuid,
    source_id: Uuid,
    max_messages: Option<usize>,
) -> Result<Option<(Conversation, Vec<ConversationMessage>)>, sqlx::Error> {
    let learner_s = learner_id.to_string();
    let source_s = source_id.to_string();
    let mut tx = pool.begin().await?;

    let source: Option<(String, Option<String>, i64, i64)> = sqlx::query_as(
        "SELECT id, title, is_archived, pinned FROM conversations WHERE id = ? AND learner_id = ?",
    )
    .bind(&source_s)
    .bind(&learner_s)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((_, title, is_archived, pinned)) = source else {
        return Ok(None);
    };

    let new_id = Uuid::new_v4();
    let now = Utc::now();
    let now_s = now.to_rfc3339();
    let new_title = match title {
        Some(t) if !t.trim().is_empty() => format!("{t} (copy)"),
        _ => "New Chat (copy)".to_string(),
    };
    sqlx::query(
        "INSERT INTO conversations (id, learner_id, title, is_archived, pinned, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(new_id.to_string())
    .bind(&learner_s)
    .bind(&new_title)
    .bind(is_archived)
    .bind(pinned)
    .bind(&now_s)
    .bind(&now_s)
    .execute(&mut *tx)
    .await?;

    let source_messages: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT role, content, created_at FROM conversation_messages \
         WHERE conversation_id = ? ORDER BY created_at ASC",
    )
    .bind(&source_s)
    .fetch_all(&mut *tx)
    .await?;
    let mut messages = Vec::with_capacity(source_messages.len());
    for (role, content, _created) in source_messages.iter().take(max_messages.unwrap_or(usize::MAX))
    {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO conversation_messages (id, conversation_id, role, content, created_at) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(new_id.to_string())
        .bind(role)
        .bind(content)
        .bind(&now_s)
        .execute(&mut *tx)
        .await?;
        messages.push(ConversationMessage {
            id,
            conversation_id: new_id,
            role: parse_role(role),
            content: content.clone(),
            created_at: now,
        });
    }

    let tags: Vec<(String,)> = sqlx::query_as(
        "SELECT tag FROM conversation_tag_map WHERE conversation_id = ? AND learner_id = ?",
    )
    .bind(&source_s)
    .bind(&learner_s)
    .fetch_all(&mut *tx)
    .await?;
    for (tag,) in &tags {
        sqlx::query(
            "INSERT INTO conversation_tag_map (conversation_id, tag, learner_id) VALUES (?, ?, ?)",
        )
        .bind(new_id.to_string())
        .bind(tag)
        .bind(&learner_s)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    Ok(Some((
        Conversation {
            id: new_id,
            learner_id,
            title: Some(new_title),
            is_archived: is_archived != 0,
            pinned: pinned != 0,
            created_at: now,
            updated_at: now,
        },
        messages,
    )))
}
