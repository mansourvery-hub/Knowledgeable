use domain::{Conversation, ConversationMessage, MessageRole};
use sqlx::SqlitePool;
use uuid::Uuid;

pub const DEFAULT_LEARNER_ID: &str = "00000000-0000-0000-0000-000000000001";

pub fn default_learner_id() -> Uuid {
    Uuid::parse_str(DEFAULT_LEARNER_ID).unwrap()
}

pub async fn ensure_default_learner(pool: &SqlitePool) -> Result<Uuid, sqlx::Error> {
    let learner_id = default_learner_id();
    infrastructure::conversation_repo::ensure_default_learner(pool, learner_id).await?;
    Ok(learner_id)
}

pub async fn create_conversation(
    pool: &SqlitePool,
    title: Option<&str>,
) -> Result<Conversation, sqlx::Error> {
    let learner_id = ensure_default_learner(pool).await?;
    let conv =
        infrastructure::conversation_repo::create_conversation(pool, learner_id, title).await?;
    Ok(conv)
}

pub async fn list_conversations(pool: &SqlitePool) -> Result<Vec<Conversation>, sqlx::Error> {
    let learner_id = default_learner_id();
    infrastructure::conversation_repo::list_conversations(pool, learner_id).await
}

pub async fn get_conversation(
    pool: &SqlitePool,
    id: Uuid,
) -> Result<Option<Conversation>, sqlx::Error> {
    let learner_id = default_learner_id();
    infrastructure::conversation_repo::get_conversation(pool, learner_id, id).await
}

pub async fn add_message(
    pool: &SqlitePool,
    conversation_id: Uuid,
    role: MessageRole,
    content: &str,
) -> Result<ConversationMessage, sqlx::Error> {
    infrastructure::conversation_repo::create_message(pool, conversation_id, role, content).await
}

pub async fn list_messages(
    pool: &SqlitePool,
    conversation_id: Uuid,
) -> Result<Vec<ConversationMessage>, sqlx::Error> {
    infrastructure::conversation_repo::list_messages(pool, conversation_id).await
}
