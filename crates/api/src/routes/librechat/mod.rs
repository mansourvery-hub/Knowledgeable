//! LibreChat frontend adapter.
//!
//! Implements the subset of `/api/*` endpoints the vendored LibreChat client
//! (`apps/web`) requires, backed natively by Knowledgeable's Rust/SQLite core.
//! No Node.js, MongoDB, Redis, or MeiliSearch is involved.

pub mod chat;
pub mod concepts;
pub mod convos;
pub mod stream_registry;
pub mod system;
pub mod wiki;

#[cfg(test)]
mod tests;

use axum::{
    routing::{get, post},
    Router,
};

use crate::routes::AppState;

/// Endpoint key advertised to the LibreChat client. Conversations store this
/// value in `endpoint`, and chat requests POST to `/api/agents/chat/{this}`.
pub const ENDPOINT_NAME: &str = "knowledgeable";

/// Sentinel `parentMessageId` LibreChat uses for a conversation's first message.
pub const NO_PARENT: &str = "00000000-0000-0000-0000-000000000000";

/// The single-user learner identity used until multi-user auth lands.
pub const LEARNER_DISPLAY_NAME: &str = "Learner";

pub fn routes() -> Router<AppState> {
    Router::new()
        // System / session
        .route("/api/config", get(system::config))
        .route("/api/user", get(system::user))
        .route("/api/auth/refresh", get(system::refresh_token).post(system::refresh_token))
        .route("/api/endpoints", get(system::endpoints))
        .route("/api/models", get(system::models))
        // Roles gate client UI (and the ChatRoute boot gate `roles.USER`).
        .route("/api/roles/:role_name", get(system::role))
        // Conversations
        .route("/api/convos", get(convos::list).delete(convos::delete))
        .route("/api/convos/update", post(convos::update))
        .route("/api/convos/gen_title/:id", get(convos::gen_title))
        .route("/api/convos/:id", get(convos::get_one))
        // Messages
        .route("/api/messages/:conversation_id", get(convos::list_messages))
        // Chat (SSE). LibreChat posts every endpoint's turns through the agents
        // router; `:endpoint` is the endpoint key returned by `/api/endpoints`.
        .route("/api/agents/chat/:endpoint", post(chat::handle))
        // Personal Knowledge Wiki (M8): cached projection of the learner graph.
        .route("/api/concepts/:id/wiki", get(wiki::handle_get_wiki))
        // Concept search for pickers (graph explorer): bounded, read-only.
        .route("/api/concepts/search", get(concepts::handle_search_concepts))
        // v2 generation protocol: the start ticket from the POST above
        // attaches to its live SSE here (resumes converge via snapshot),
        // and terminal teardown is authorized through the status read.
        .route("/api/agents/chat/stream/:stream_id", get(chat::handle_stream))
        .route("/api/agents/chat/status/:conversation_id", get(chat::handle_status))
}

/// Serializes a conversation into LibreChat's `TConversation` shape.
pub fn conv_json(conv: &domain::Conversation) -> serde_json::Value {
    serde_json::json!({
        "conversationId": conv.id.to_string(),
        "endpoint": ENDPOINT_NAME,
        // Fallback validation schema for the client's `parseConvo`: our
        // endpoint key is unknown to its schema registry, so without this
        // `buildDefaultConvo` throws and chat boot dies. `custom` selects the
        // plain chat shape, which matches this adapter (text in/out, params
        // owned server-side). The `endpoint` key itself is unchanged, so chat
        // turns still route to `/api/agents/chat/knowledgeable`.
        "endpointType": "custom",
        "title": conv.title.clone().unwrap_or_else(|| "New Chat".to_string()),
        "createdAt": conv.created_at.to_rfc3339(),
        "updatedAt": conv.updated_at.to_rfc3339(),
        "isArchived": false,
        "pinned": false,
    })
}

/// Serializes a message into LibreChat's `TMessage` shape.
pub fn msg_json(
    message: &domain::ConversationMessage,
    parent_id: &str,
    model: Option<&str>,
) -> serde_json::Value {
    let is_user = matches!(message.role, domain::MessageRole::User);
    serde_json::json!({
        "messageId": message.id.to_string(),
        "conversationId": message.conversation_id.to_string(),
        "parentMessageId": parent_id,
        "text": message.content,
        "sender": if is_user { "User" } else { "Knowledgeable" },
        "isCreatedByUser": is_user,
        "endpoint": ENDPOINT_NAME,
        "model": model,
        "createdAt": message.created_at.to_rfc3339(),
        "updatedAt": message.created_at.to_rfc3339(),
        "title": "New Chat",
        "error": false,
    })
}

/// Serializes an assistant reply that may not be persisted yet (empty replies).
#[allow(clippy::too_many_arguments)]
pub fn assistant_msg_json(
    message_id: uuid::Uuid,
    conversation_id: uuid::Uuid,
    parent_id: &str,
    text: &str,
    model: Option<&str>,
    created_at: chrono::DateTime<chrono::Utc>,
    error: bool,
) -> serde_json::Value {
    serde_json::json!({
        "messageId": message_id.to_string(),
        "conversationId": conversation_id.to_string(),
        "parentMessageId": parent_id,
        "text": text,
        "sender": "Knowledgeable",
        "isCreatedByUser": false,
        "endpoint": ENDPOINT_NAME,
        "model": model,
        "createdAt": created_at.to_rfc3339(),
        "updatedAt": created_at.to_rfc3339(),
        "title": "New Chat",
        "error": error,
    })
}

/// Derives a short conversation title from its first user message.
pub fn derive_title(text: &str) -> String {
    const MAX: usize = 48;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return "New Chat".to_string();
    }
    let mut title: String = trimmed.chars().take(MAX).collect();
    if trimmed.chars().count() > MAX {
        title.push('…');
    }
    title
}
