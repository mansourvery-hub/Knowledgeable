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

pub async fn update_conversation_title(
    pool: &SqlitePool,
    conversation_id: Uuid,
    title: &str,
) -> Result<(), sqlx::Error> {
    infrastructure::conversation_repo::update_conversation_title(pool, conversation_id, title).await
}

pub async fn delete_conversation(
    pool: &SqlitePool,
    conversation_id: Uuid,
) -> Result<(), sqlx::Error> {
    infrastructure::conversation_repo::delete_conversation(pool, conversation_id).await
}

/// Lists conversations with optional archived/pinned filters (Phase 5
/// pin/archive): the sidebar archive view and pinned-section drain.
pub async fn list_conversations_filtered(
    pool: &SqlitePool,
    archived: Option<bool>,
    pinned: Option<bool>,
) -> Result<Vec<Conversation>, sqlx::Error> {
    let learner_id = default_learner_id();
    infrastructure::conversation_repo::list_conversations_filtered(
        pool, learner_id, archived, pinned,
    )
    .await
}

/// Sets the archived flag. Returns `false` for unknown conversations.
pub async fn set_archived(
    pool: &SqlitePool,
    conversation_id: Uuid,
    archived: bool,
) -> Result<bool, sqlx::Error> {
    let learner_id = default_learner_id();
    infrastructure::conversation_repo::set_archived(pool, learner_id, conversation_id, archived)
        .await
}

/// Sets the pinned flag. Returns `false` for unknown conversations.
pub async fn set_pinned(
    pool: &SqlitePool,
    conversation_id: Uuid,
    pinned: bool,
) -> Result<bool, sqlx::Error> {
    let learner_id = default_learner_id();
    infrastructure::conversation_repo::set_pinned(pool, learner_id, conversation_id, pinned).await
}

/// Archives every unarchived conversation; returns the archived count.
pub async fn archive_all(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    let learner_id = default_learner_id();
    infrastructure::conversation_repo::archive_all(pool, learner_id).await
}

/// F20: synthesized conversation titles.
///
/// Display cap mirrors the API's `derive_title` truncation so a synthesized
/// title never renders differently from a provisional one.
pub const TITLE_MAX_CHARS: usize = 48;

/// Builds the title-writing prompt from the first exchange. Both sides are
/// bounded so one long reply cannot blow up the prompt; the reply is what
/// grounds "what the tab is about" beyond the raw prompt echo.
pub fn title_prompt(user_text: &str, assistant_text: &str) -> String {
    fn clip(s: &str) -> String {
        const MAX: usize = 300;
        let t = s.trim();
        if t.chars().count() > MAX {
            let mut c: String = t.chars().take(MAX).collect();
            c.push('…');
            c
        } else {
            t.to_string()
        }
    }
    format!(
        "Write a short tab title (at most six words) naming what this conversation is about. \
         Reply with JSON only: {{\"title\": \"...\"}}. Learner asked: \"{}\". Tutor answered: \"{}\".",
        clip(user_text),
        clip(assistant_text),
    )
}

/// Cleans a model-written title for sidebar/document display: single line,
/// no quotes or markdown emphasis, capped at [`TITLE_MAX_CHARS`].
/// Returns `None` when nothing usable remains (caller keeps truncation).
pub fn sanitize_title(raw: &str) -> Option<String> {
    let single_line: String = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let stripped = single_line
        .trim()
        .trim_matches(|c| c == '"' || c == '\'' || c == '*' || c == '_' || c == '#' || c == '`')
        .trim();
    if stripped.is_empty() {
        return None;
    }
    let mut title: String = stripped.chars().take(TITLE_MAX_CHARS).collect();
    if stripped.chars().count() > TITLE_MAX_CHARS {
        title.push('…');
    }
    Some(title)
}

/// Generates a synthesized title for a fresh conversation and persists it.
/// Returns `true` when a model-written title was stored, `false` when the
/// caller should keep the provisional truncation: empty assistant text
/// (error turns must not title from error copy), any LLM failure, or an
/// unusable draft. Never errors on LLM trouble — titles are cosmetic.
pub async fn synthesize_and_store_title(
    pool: &SqlitePool,
    llm: &std::sync::Arc<dyn llm::LlmClient>,
    model: &str,
    conversation_id: Uuid,
    user_text: &str,
    assistant_text: &str,
) -> bool {
    if assistant_text.trim().is_empty() {
        return false;
    }
    let prompt = title_prompt(user_text, assistant_text);
    let draft = match llm
        .generate_structured(llm::LlmStructuredRequest {
            model: model.to_string(),
            prompt,
            schema: serde_json::json!({
                "type": "object",
                "properties": { "title": { "type": "string" } },
                "required": ["title"],
            }),
        })
        .await
    {
        Ok(value) => value,
        Err(e) => {
            tracing::warn!(error = %e, "title synthesis failed, keeping truncation");
            return false;
        }
    };
    let raw = draft.get("title").and_then(|t| t.as_str()).unwrap_or("");
    match sanitize_title(raw) {
        Some(title) => {
            if let Err(e) = update_conversation_title(pool, conversation_id, &title).await {
                tracing::warn!(error = %e, "title synthesis persist failed");
                return false;
            }
            true
        }
        None => {
            tracing::warn!("title synthesis draft unusable, keeping truncation");
            false
        }
    }
}
