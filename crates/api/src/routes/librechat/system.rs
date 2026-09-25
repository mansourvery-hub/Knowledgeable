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
    let mut config = json!({
        "appTitle": "Knowledgeable",
        "emailLoginEnabled": false,
        "registrationEnabled": false,
        "socialLoginEnabled": false,
        "passwordResetEnabled": false,
        "emailEnabled": false,
        "showBirthdayIcon": false,
        "helpAndFaqURL": "",
        "sharedLinksEnabled": true,
        "publicSharedLinksEnabled": true,
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
    });
    // Brand levers the client already honors (Phase 5): an empty custom
    // footer suppresses the generic LibreChat footer line, and customWelcome
    // replaces the landing greeting. Inserted here (not in the literal) to
    // stay under the `json!` macro recursion limit.
    config["customFooter"] = json!("");
    config["interface"]["customWelcome"] = json!("What do you want to understand?");
    Json(config)
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
/// configured, always plus the offline `local-tutor` fallback.
pub async fn models() -> Json<Value> {
    Json(json!({
        ENDPOINT_NAME: application::llm_dispatch::advertised_models(),
    }))
}

/// `GET /api/search/enable` — conversation search availability (Phase 5).
/// Served `true`: the bundled SQLite substring search backs both the
/// sidebar filter and the `/search` page, so no MeiliSearch daemon gates it.
pub async fn search_enabled() -> Json<bool> {
    Json(true)
}

/// Quiet stubs for removed/future surfaces (M1 box 1): the vendored client
/// probes these on every boot and logs a handled Axios 404/405 rejection per
/// miss. Each stub returns the payload that keeps the corresponding UI
/// exactly as it renders today (hidden/empty), so the only observable change
/// is fewer boot requests and a clean console. No vendored code is touched;
/// when a surface becomes a real Phase 5 project, its stub is replaced by
/// the contract implementation.
/// Deferred (behavioral, not noise): `speech/config` (a 200 flips
/// `speechSettingsInitialized` false→true — needs its own parity brick) and
/// `user/settings/{favorites,pinned-order}` (read-your-writes persistence
/// semantics — needs its own brick).
/// `GET /api/banner` — no banner: `Banner.tsx` renders null on falsy data.
pub async fn banner_none() -> Json<Value> {
    Json(Value::Null)
}

/// `GET /api/files` — no files: empty list; the FilesPanel entry is already
/// gated by our `attachmentsDisabled` flag.
pub async fn files_empty() -> Json<Value> {
    Json(json!([]))
}

/// `GET /api/files/config` — no upload configuration: every field of the
/// client schema is optional, and `useUploadOptions` forces `uploadsDisabled`
/// regardless, so `{}` keeps paste/drag/modal on the disabled toast.
pub async fn files_config_empty() -> Json<Value> {
    Json(json!({}))
}

/// `GET /api/keys?name=*` — no BYOK key: `{expiresAt: ""}` is the client's
/// own established "no key" value (its queryFn returns it for empty names).
pub async fn user_key_absent() -> Json<Value> {
    Json(json!({ "expiresAt": "" }))
}

/// `GET /api/agents/tools/:tool_id/auth` — tool unauthenticated:
/// `useToolToggle` reads `data?.authenticated ?? false`, so `false`
/// preserves the 404 behavior exactly.
pub async fn tool_auth_denied() -> Json<Value> {
    Json(json!({ "authenticated": false }))
}

/// `GET /api/agents/chat/active` — no active runs: `useActiveJobs` polls
/// with `retry: false`, so `[]` means nothing to resume (same as today's
/// 405, without the rejection).
pub async fn active_jobs_empty() -> Json<Value> {
    Json(json!([]))
}

/// `GET /api/balance` — no token balance: every consumer is gated on
/// `startupConfig.balance.enabled` (absent from our config), so this payload
/// is currently unread; zero credits keeps it honest if gating ever lapses.
pub async fn balance_zero() -> Json<Value> {
    Json(json!({ "tokenCredits": 0, "autoRefillEnabled": false }))
}

/// `GET /api/files/speech/config/get` — speech unconfigured (M1 brick 2a).
/// The init hook treats `{message: "not_found"}` as "no server defaults":
/// it skips the defaults loop, changes no engine, and marks settings
/// initialized. All three `speechSettingsInitialized` consumers
/// (`AudioRecorder`, `AutoPlayAudio`, `MessageAudio`) sit behind the
/// `speechDisabled` render gates, so the false→true flip is UI-inert —
/// verified by the browser parity proof, not just by reading.
pub async fn speech_config_unconfigured() -> Json<Value> {
    Json(json!({ "message": "not_found" }))
}

/// `GET /api/user/settings/favorites` — no favorites (M1 brick 2b). The
/// `useFavorites` effect writes the atom only when query data arrives, so
/// `[]` renders exactly what today's 404 renders: nothing. Writes still
/// 404 (no persistence store), unchanged from today.
///
/// Deliberately NOT stubbed: `GET /api/user/settings/pinned-order`. The
/// PinnedSection merge treats a *successful* fetch as authoritative server
/// state, so serving `[]` would enable the drag/Alt+Arrow reorder path
/// whose POST has no backend — worse than today's cleanly-gated 404.
/// Silencing it properly means a real persistence brick (settings store +
/// GET/POST), which is its own feature, not a quiet stub.
pub async fn favorites_empty() -> Json<Value> {
    Json(json!([]))
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
    // No "TEMPORARY_CHAT": ephemeral chats are outside the product surface
    // (Phase 1: `interface.temporaryChat` already false). Verified safe: the
    // header toggle/indicator, header-menu item, and keyboard shortcut are
    // strict conditionals on the grant, and our backend never returns
    // `isTemporary`/`expiredAt` so the normal path is untouched. Known
    // follow-up: the `defaultTemporaryChat` settings toggle has no `show`
    // gate upstream — hiding it needs a small frontend seam of its own.
    // No "MULTI_CONVO": side-by-side compare is outside the product surface
    // (Phase 1: `interface.multiConvo` already false). Verified safe: the
    // header button, header-menu item, `+` popover handler, and settings
    // toggle are all strict conditional renders / early-returns on the grant.
    // No "RUN_CODE": executing code is a FUTURE/OPTIONAL capability while
    // displaying code stays KEEP (policy §9.4). Verified safe: `canRunCode`
    // feeds only `allowExecution` on `CodeBlock` — highlighting, copy,
    // Mermaid, and math rendering are untouched; the badge-row toggle,
    // tools-dropdown row, and code-workspace surfaces hide centrally.
    // No "WEB_SEARCH": external retrieval is a FUTURE/OPTIONAL capability,
    // never a hidden dependency of the core tutor (policy §9.4). The grant
    // pairs with agent capabilities at every consumer (`WebSearch.tsx`
    // returns null without it; tools-dropdown rows require
    // `canUseWebSearch && webSearchEnabled`) — so revocation hides centrally.
    // No "PEOPLE_PICKER": principal picking serves per-resource sharing
    // dialogs with no backend here (policy §9.3); the future minimal
    // "share this conversation" is link-based (SHARED_LINKS held), not
    // principal-based. Already half-denied today — our role never granted
    // the VIEW_USERS/GROUPS/ROLES sub-permissions — so revoking the type
    // entry only hides the admin section and keeps the set honest.
    // No "MARKETPLACE": the agent marketplace is outside the product surface
    // (policy §9.3). The client was built for this gate — `Marketplace.tsx`
    // renders null and redirects to `/c/new` without USE, and
    // `useShowMarketplace` already reads false (AGENTS revoked earlier).
    // No "FILE_SEARCH": document RAG is a FUTURE capability for
    // learner-provided material (policy §9.4). `FileSearch.tsx` returns null
    // without the grant and tools-dropdown rows pair it with capabilities —
    // so revocation hides centrally; no `/api/files/*` backend exists today.
    // No "FILE_CITATIONS": dead grant — no client code gates on this
    // permission (only the unrelated `FileCitation` data type exists).
    // Removing it keeps the granted set minimal and honest.
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
    // No "SCHEDULES": scheduled chats are outside the product surface
    // (policy §9.3). All three consumers live inside the Schedules surface
    // itself (side-panel entry already hidden via the absent interface flag;
    // panel and card) — revocation double-locks it centrally.
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
