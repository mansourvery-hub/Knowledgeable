//! System, session, endpoint, and model exposure for the LibreChat client.

use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::{json, Value};

use super::ENDPOINT_NAME;
use crate::routes::AppState;

/// `GET /api/config` — startup configuration.
///
/// Single-user local mode: auth is effectively disabled (the refresh endpoint
/// always mints a local session), no social logins, and only the
/// Knowledgeable endpoint is advertised.
pub async fn config(State(_state): State<AppState>) -> Json<Value> {
    Json(json!({
        "appTitle": "Knowledgeable",
        "emailLoginEnabled": false,
        "registrationEnabled": false,
        "socialLoginEnabled": false,
        "passwordResetEnabled": false,
        "emailEnabled": false,
        "showBirthdayIcon": false,
        "helpAndFaqURL": "",
        "sharedLinksEnabled": false,
        "publicSharedLinksEnabled": false,
        "allowAccountDeletion": false,
        "discordLoginEnabled": false,
        "facebookLoginEnabled": false,
        "githubLoginEnabled": false,
        "googleLoginEnabled": false,
        "openidLoginEnabled": false,
        "appleLoginEnabled": false,
        "samlLoginEnabled": false,
        "openidLabel": "",
        "openidImageUrl": "",
        "openidAutoRedirect": false,
        "samlLabel": "",
        "samlImageUrl": "",
        "serverDomain": "localhost",
        "titleGenerationTiming": "final",
        "interface": {
            "modelSelect": true,
            "parameters": false,
            "sidePanel": true,
            "presets": false,
            "prompts": false,
            "bookmarks": false,
            "multiConvo": false,
            "agents": false,
            "temporaryChat": false,
            "runCode": false,
            "webSearch": false,
            "fileSearch": false,
            "contextUsage": false,
            "feedback": false,
        },
        "endpoints": {
            ENDPOINT_NAME: endpoint_config(),
        }
    }))
}

/// `GET /api/user` — current learner profile.
pub async fn user() -> Json<Value> {
    Json(user_json())
}

/// `POST|GET /api/auth/refresh` — mints a local session for the single learner.
pub async fn refresh_token() -> Json<Value> {
    Json(json!({
        "token": "local-session-token",
        "user": user_json(),
    }))
}

/// `GET /api/endpoints` — advertised chat endpoints.
pub async fn endpoints() -> Json<Value> {
    Json(json!({
        ENDPOINT_NAME: endpoint_config(),
    }))
}

/// `GET /api/models` — models available per endpoint.
///
/// Truthful per M3: provider models appear only when their server key is
/// configured, the Ollama model only when explicitly enabled, always plus
/// the offline `local-tutor` fallback the header picker can always use.
pub async fn models() -> Json<Value> {
    Json(json!({
        ENDPOINT_NAME: application::llm_dispatch::advertised_models(),
    }))
}

/// Permission types the client gates UI on (`PermissionTypes` in
/// `librechat-data-provider`). Kept as a literal list so a data-provider
/// upgrade that adds a type fails loudly here instead of silently hiding UI.
const LOCAL_PERMISSION_TYPES: &[&str] = &[
    // No "PROMPTS": prompt library management is outside the product surface
    // and slash commands are a disabled FUTURE (policy §9.4). Revoking the
    // grant hides the Prompts panel, the slash-command setting, and the
    // chat-input slash popover centrally (`useHasAccess` strict `=== true`).
    // No "AGENTS": agent builder/marketplace are outside the product surface
    // (policy §9.3). Verified safe: `useEndpoints` only filters the `agents`
    // endpoint (ours is `knowledgeable`); `useNewConvo` falls through to the
    // non-agents endpoint; `useProviderKeys` already excludes agent providers
    // when `agents` is unconfigured. The `/agents` route itself is a later
    // Phase 2 route-gating step.
    "BOOKMARKS",
    // No "MEMORIES": generic LibreChat memory competes with the authoritative
    // learner graph (policy §9.6). Absent grants read as denied client-side
    // (`useHasAccess` strict `=== true`), hiding the Memories panel, its
    // settings toggle, and chat-input memory affordances centrally.
    "MULTI_CONVO",
    "TEMPORARY_CHAT",
    "RUN_CODE",
    "WEB_SEARCH",
    "PEOPLE_PICKER",
    // No "MARKETPLACE": the agent marketplace is outside the product surface
    // (policy §9.3). The client was built for this gate — `Marketplace.tsx`
    // renders null and redirects to `/c/new` without USE, and
    // `useShowMarketplace` already reads false (AGENTS revoked earlier).
    "FILE_SEARCH",
    "FILE_CITATIONS",
    // No "MCP_SERVERS": MCP UI is hidden for now while the future tutor-tool
    // seam is preserved (policy §9.4). The client was built for this gate —
    // `useAppStartup` suppresses all MCP queries without USE, and the side
    // panel, tools-dropdown entry, and dialogs hide centrally.
    // No "REMOTE_AGENTS": remote-agent sharing and generic agent API-key
    // management are outside the product surface (policy §9.3). Revoking
    // hides the settings API-keys entry, the admin permission editor, and
    // the agent-footer share action centrally; nothing in the chat turn path
    // reads this grant.
    // No "SKILLS": generic skills management is outside the product surface
    // (policy §9.3). Every consumer pairs the grant with agent capabilities
    // or lives inside removed surfaces, and the `$` mention handler
    // early-returns without access — so revocation hides centrally.
    "SHARED_LINKS",
    "SCHEDULES",
];

/// `GET /api/roles/:role_name` — local role grants.
///
/// Single-user local mode has no role administration: the USER role holds
/// every capability the client gates on, which is also what unblocks the
/// chat boot gate (`roles.USER != null` in `ChatRoute`). Unknown role names
/// 404 so typos surface instead of silently degrading to hidden UI.
pub async fn role(Path(name): Path<String>) -> Result<Json<Value>, crate::error::AppError> {
    if !name.eq_ignore_ascii_case("USER") {
        return Err(crate::error::AppError::NotFound("role not found".into()));
    }
    let mut permissions = serde_json::Map::with_capacity(LOCAL_PERMISSION_TYPES.len());
    for permission_type in LOCAL_PERMISSION_TYPES {
        permissions.insert(
            (*permission_type).to_string(),
            json!({
                "USE": true,
                "CREATE": true,
                "UPDATE": true,
                "READ": true,
                "READ_AUTHOR": true,
                "SHARE": true,
                "OPT_OUT": true,
            }),
        );
    }
    Ok(Json(json!({ "name": "USER", "permissions": Value::Object(permissions) })))
}

fn endpoint_config() -> Value {
    json!({
        "order": 0,
        "name": "Knowledgeable",
        // Marks this endpoint as a custom plain-chat provider so the client
        // resolves its `parseConvo` fallback schema (see `conv_json`).
        "type": "custom",
    })
}

fn user_json() -> Value {
    let id = application::conversation_service::DEFAULT_LEARNER_ID;
    json!({
        "id": id,
        "username": "learner",
        "email": "learner@local",
        "name": super::LEARNER_DISPLAY_NAME,
        "avatar": "",
        "role": "USER",
        "provider": "local",
        "createdAt": "2026-01-01T00:00:00.000Z",
        "updatedAt": "2026-01-01T00:00:00.000Z",
    })
}
