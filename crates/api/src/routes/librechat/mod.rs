//! LibreChat frontend adapter.
//!
//! Implements the subset of `/api/*` endpoints the vendored LibreChat client
//! (`apps/web`) requires, backed natively by Knowledgeable's Rust/SQLite core.
//! No Node.js, MongoDB, Redis, or MeiliSearch is involved.

pub mod chat;
pub mod concepts;
pub mod convos;
pub mod share;
pub mod stream_registry;
pub mod system;
pub mod tags;
pub mod wiki;

#[cfg(test)]
mod tests;

use axum::{
    routing::{get, post, put},
    Router,
};

use crate::routes::AppState;
use crate::routes::RateLimiter;

/// Endpoint key advertised to the LibreChat client. Conversations store this
/// value in `endpoint`, and chat requests POST to `/api/agents/chat/{this}`.
pub const ENDPOINT_NAME: &str = "knowledgeable";

/// Sentinel `parentMessageId` LibreChat uses for a conversation's first message.
pub const NO_PARENT: &str = "00000000-0000-0000-0000-000000000000";

/// The single-user learner identity used until multi-user auth lands.
pub const LEARNER_DISPLAY_NAME: &str = "Learner";

pub fn routes(chat_limiter: Option<RateLimiter>) -> Router<AppState> {
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
        .route("/api/convos/archive", post(convos::archive))
        .route("/api/convos/archive/all", post(convos::archive_all))
        .route("/api/convos/pin", post(convos::pin))
        .route("/api/convos/duplicate", post(convos::duplicate))
        .route("/api/convos/:id", get(convos::get_one))
        // Conversation tags for bookmarks (Phase 5, F19): the static
        // `convo` segment wins over `:tag` so the two cannot collide.
        .route("/api/tags", get(tags::list).post(tags::create))
        .route("/api/tags/convo/:conversation_id", put(tags::set_for_conversation))
        .route("/api/tags/:tag", put(tags::update).delete(tags::delete))
        // Messages
        .route("/api/messages/:conversation_id", get(convos::list_messages))
        .route("/api/messages/:conversation_id/:message_id", get(convos::get_message))
        // Global message search for the `/search` page (Phase 5): exact
        // path wins over `:conversation_id`, so the two cannot collide.
        .route("/api/messages", get(convos::search_messages))
        // Search availability flag for the sidebar filter + `/search` page.
        .route("/api/search/enable", get(system::search_enabled))
        // Quiet stubs for removed/future surfaces (M1 box 1): same UI as
        // today's handled 404/405s, without the boot request burst or the
        // console rejections. Static segments, no capture collisions.
        .route("/api/banner", get(system::banner_none))
        .route("/api/files", get(system::files_empty))
        .route("/api/files/config", get(system::files_config_empty))
        .route("/api/keys", get(system::user_key_absent))
        .route("/api/agents/tools/:tool_id/auth", get(system::tool_auth_denied))
        .route("/api/agents/chat/active", get(system::active_jobs_empty))
        .route("/api/balance", get(system::balance_zero))
        // Chat (SSE). LibreChat posts every endpoint's turns through the agents
        // router; `:endpoint` is the endpoint key returned by `/api/endpoints`.
        // The spend path carries its own per-key budget (no-op when unset).
        .route(
            "/api/agents/chat/:endpoint",
            post(chat::handle).route_layer(axum::middleware::from_fn_with_state(
                chat_limiter,
                crate::routes::chat_rate_limit,
            )),
        )
        // Personal Knowledge Wiki (M8): cached projection of the learner graph.
        .route("/api/concepts/:id/wiki", get(wiki::handle_get_wiki))
        // Concept search for pickers (graph explorer): bounded, read-only.
        .route("/api/concepts/search", get(concepts::handle_search_concepts))
        // Mastered concepts for the wiki browser (Phase 5a, W1): bounded,
        // weakest-first, with wiki page state. Static segment, so it cannot
        // collide with `/api/concepts/:id/wiki` (same as `/search` above).
        .route("/api/concepts/mastered", get(concepts::handle_list_mastered))
        // v2 generation protocol: the start ticket from the POST above
        // attaches to its live SSE here (resumes converge via snapshot),
        // and terminal teardown is authorized through the status read.
        .route("/api/agents/chat/stream/:stream_id", get(chat::handle_stream))
        .route("/api/agents/chat/status/:conversation_id", get(chat::handle_status))
        // Minimal conversation sharing (Phase 5, F27): one capture name
        // (`:id`) across the dynamic share routes — Axum rejects differing
        // names at the same position. Static `link`/`config` segments and
        // the method split disambiguate the rest.
        .route("/api/share/link/:conversation_id", get(share::get_for_conversation))
        .route("/api/share/:id", post(share::create))
        .route("/api/share/:id", get(share::read).patch(share::update).delete(share::delete))
        .route("/api/share/:id/config", get(share::render_config))
        .route("/api/share/:id/fork", post(share::fork))
}

/// Serializes a conversation with its bookmark tags. The client reads
/// per-conversation `tags` for menu state and cache reconciliation, so list
/// and detail handlers pass the live membership (fetched in one round-trip
/// via `tags_for_conversations`); fresh conversations carry none.
pub fn conv_json_tags(conv: &domain::Conversation, tags: &[String]) -> serde_json::Value {
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
        "tags": tags,
        "createdAt": conv.created_at.to_rfc3339(),
        "updatedAt": conv.updated_at.to_rfc3339(),
        "isArchived": conv.is_archived,
        "pinned": conv.pinned,
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
