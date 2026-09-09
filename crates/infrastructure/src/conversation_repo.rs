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
        created_at: now,
        updated_at: now,
    })
}

pub async fn list_conversations(
    pool: &SqlitePool,
    learner_id: Uuid,
) -> Result<Vec<Conversation>, sqlx::Error> {
    let learner_s = learner_id.to_string();
    let rows = sqlx::query_as::<_, (String, String, Option<String>, String, String)>(
        "SELECT id, learner_id, title, created_at, updated_at FROM conversations WHERE learner_id = ? ORDER BY updated_at DESC",
    )
    .bind(&learner_s)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, learner_id, title, created_at, updated_at)| Conversation {
            id: id.parse().unwrap(),
            learner_id: learner_id.parse().unwrap(),
            title,
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
    let row = sqlx::query_as::<_, (String, String, Option<String>, String, String)>(
        "SELECT id, learner_id, title, created_at, updated_at FROM conversations WHERE id = ? AND learner_id = ?",
    )
    .bind(&id_s)
    .bind(&learner_s)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(id, learner_id, title, created_at, updated_at)| Conversation {
        id: id.parse().unwrap(),
        learner_id: learner_id.parse().unwrap(),
        title,
        created_at: parse_dt(&created_at),
        updated_at: parse_dt(&updated_at),
    }))
}

pub async fn create_message(
    pool: &SqlitePool,
    conversation_id: Uuid,
    role: MessageRole,
    content: &str,
) -> Result<ConversationMessage, sqlx::Error> {
    let id = Uuid::new_v4();
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
