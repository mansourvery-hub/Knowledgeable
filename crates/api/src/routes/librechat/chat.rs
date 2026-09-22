//! `POST /api/agents/chat/:endpoint` — LibreChat SSE streaming adapter.
//!
//! Two wire modes share one turn driver:
//!
//! - **Legacy SSE** (default): streams `created`, cumulative `text` deltas,
//!   an optional `concept_annotations` frame, and terminal `final` directly
//!   on the POST response. Used by curl, tests, and assistants endpoints.
//! - **v2 generation protocol** (request carries header
//!   `X-LibreChat-Generation-Protocol: 2` or body
//!   `generationProtocolVersion: 2`): the POST only starts the turn and
//!   returns a JSON ticket `{ generationProtocolVersion: 2, status: "stream",
//!   streamId, conversationId, generationCreatedAt }`. The client then
//!   attaches via `GET /api/agents/chat/stream/:stream_id` (see
//!   [`handle_stream`]), which replays the snapshot and forwards live
//!   frames, so reconnects with `?resume=true` converge.
//!
//! Frame shapes are identical in both modes (built by the `*_frame`
//! constructors), including shapes the vendored client ignores safely.

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::convert::Infallible;
use tokio_stream::Stream;
use uuid::Uuid;

use super::{
    assistant_msg_json, conv_json, derive_title, msg_json,
    stream_registry::{latest_for_conversation, new_registry_entry, StreamRegistry},
    ENDPOINT_NAME, NO_PARENT,
};
use crate::error::AppError;
use crate::routes::AppState;

pub const GENERATION_PROTOCOL_V2: i64 = 2;
const GENERATION_PROTOCOL_HEADER: &str = "x-librechat-generation-protocol";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatPayload {
    pub conversation_id: Option<String>,
    pub parent_message_id: Option<String>,
    pub text: Option<String>,
    pub model: Option<String>,
    pub generation_protocol_version: Option<i64>,
    /// Bring-your-own-key (M3): turn-scoped credential, never stored or
    /// logged. Takes precedence over server keys for the resolved provider.
    pub api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamQuery {
    pub resume: Option<bool>,
    pub generation_created_at: Option<i64>,
}

fn sse_data(value: &Value) -> Event {
    Event::default().data(value.to_string())
}

fn is_v2_start(headers: &HeaderMap, payload: &ChatPayload) -> bool {
    if payload.generation_protocol_version == Some(GENERATION_PROTOCOL_V2) {
        return true;
    }
    headers
        .get(GENERATION_PROTOCOL_HEADER)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.trim() == "2")
}

async fn pool(state: &AppState) -> Result<&sqlx::SqlitePool, AppError> {
    state.pool.as_ref().ok_or_else(|| AppError::Internal("no db pool".into()))
}

/// Resolves the target conversation, creating one when the client sends the
/// LibreChat new-conversation sentinels (`new`, `PENDING`, empty, or absent).
async fn resolve_conversation(
    pool: &sqlx::SqlitePool,
    conversation_id: Option<&str>,
) -> Result<domain::Conversation, AppError> {
    match conversation_id {
        Some(raw) if !raw.is_empty() && raw != "new" && raw != "PENDING" => {
            let id = Uuid::parse_str(raw)
                .map_err(|_| AppError::Validation(format!("invalid conversationId: {raw}")))?;
            application::conversation_service::get_conversation(pool, id)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?
                .ok_or_else(|| AppError::NotFound("conversation not found".into()))
        }
        _ => application::conversation_service::create_conversation(pool, None)
            .await
            .map_err(|e| AppError::Internal(e.to_string())),
    }
}

fn created_frame(
    user_msg: &domain::ConversationMessage,
    request_parent: &str,
    model: Option<&str>,
) -> Value {
    json!({
        "created": true,
        "message": msg_json(user_msg, request_parent, model),
    })
}

#[allow(clippy::too_many_arguments)]
fn delta_frame(
    accumulated: &str,
    initial: bool,
    conversation_id: Uuid,
    assistant_id: Uuid,
    user_msg_id: Uuid,
) -> Value {
    json!({
        "text": accumulated,
        "message": true,
        "initial": initial,
        "conversationId": conversation_id.to_string(),
        "messageId": assistant_id.to_string(),
        "parentMessageId": user_msg_id.to_string(),
        "sender": "Knowledgeable",
        "isCreatedByUser": false,
        "endpoint": ENDPOINT_NAME,
    })
}

fn annotations_frame(
    annotations: &[domain::ConceptAnnotation],
    assistant_id: Uuid,
    conversation_id: Uuid,
) -> Value {
    json!({
        "concept_annotations": annotations,
        "messageId": assistant_id.to_string(),
        "conversationId": conversation_id.to_string(),
    })
}

/// M4 progress frame for tool execution. Shaped without `text`/`message`/
/// `event` keys (like `concept_annotations`) so both client dispatchers
/// ignore it safely until the UI learns to render tool activity.
fn tool_progress_frame(
    phase: &str,
    tool_name: &str,
    call_id: &str,
    assistant_id: Uuid,
    conversation_id: Uuid,
) -> Value {
    json!({
        "tool_progress": {
            "phase": phase,
            "tool_name": tool_name,
            "call_id": call_id,
        },
        "messageId": assistant_id.to_string(),
        "conversationId": conversation_id.to_string(),
    })
}

#[allow(clippy::too_many_arguments)]
fn final_frame(
    user_msg: &domain::ConversationMessage,
    request_parent: &str,
    model: Option<&str>,
    final_conv: &domain::Conversation,
    assistant_id: Uuid,
    conversation_id: Uuid,
    response_text: &str,
    error_flag: bool,
    now: chrono::DateTime<chrono::Utc>,
) -> Value {
    json!({
        "final": true,
        "conversation": conv_json(final_conv),
        "title": final_conv.title.clone().unwrap_or_else(|| "New Chat".to_string()),
        "requestMessage": msg_json(user_msg, request_parent, model),
        "responseMessage": assistant_msg_json(
            assistant_id,
            conversation_id,
            &user_msg.id.to_string(),
            response_text,
            model,
            now,
            error_flag,
        ),
    })
}

/// Shared turn outcome: the completed assistant text plus whether the turn
/// errored. Annotation emission and `final` construction build on this.
struct TurnOutcome {
    accumulated: String,
    errored: bool,
    error_text: String,
}

/// Everything a turn driver needs besides the live event receiver.
struct TurnContext {
    pool: sqlx::SqlitePool,
    conversation: domain::Conversation,
    user_msg: domain::ConversationMessage,
    request_parent: String,
    model: Option<String>,
    assistant_id: Uuid,
    /// F20: the turn's LLM client (for background title synthesis) and the
    /// effective model name. `synthesize_title` is true only when this turn
    /// created the conversation (title was empty at turn start), so exactly
    /// the first turn synthesizes — user renames are never overwritten.
    llm: std::sync::Arc<dyn llm::LlmClient>,
    title_model: String,
    synthesize_title: bool,
}

/// Drives one tutor turn to completion, pushing every frame through `emit`
/// in stream order: `created`, cumulative `text` deltas, optional
/// `concept_annotations`, terminal `final`.
async fn drive_turn(
    ctx: TurnContext,
    mut rx: tokio::sync::mpsc::Receiver<Result<domain::TutorEvent, anyhow::Error>>,
    emit: impl Fn(Value),
) {
    let conversation_id = ctx.conversation.id;
    emit(created_frame(&ctx.user_msg, &ctx.request_parent, ctx.model.as_deref()));

    let mut accumulated = String::new();
    let mut initial = true;
    let mut errored = false;
    let mut error_text = String::new();

    while let Some(event) = rx.recv().await {
        match event {
            Ok(domain::TutorEvent::TextDelta { text }) => {
                accumulated.push_str(&text);
                emit(delta_frame(
                    &accumulated,
                    initial,
                    conversation_id,
                    ctx.assistant_id,
                    ctx.user_msg.id,
                ));
                initial = false;
            }
            Ok(domain::TutorEvent::Error { code, message }) => {
                errored = true;
                error_text = "The tutor could not complete this turn.".to_string();
                tracing::warn!(code = %code, message = %message, "tutor turn error");
                break;
            }
            Ok(domain::TutorEvent::TurnCompleted { .. }) => break,
            Ok(domain::TutorEvent::ToolCallStarted { tool_name, call_id }) => {
                emit(tool_progress_frame(
                    "started",
                    &tool_name,
                    &call_id,
                    ctx.assistant_id,
                    conversation_id,
                ));
            }
            Ok(domain::TutorEvent::ToolCallFinished { tool_name, call_id }) => {
                emit(tool_progress_frame(
                    "finished",
                    &tool_name,
                    &call_id,
                    ctx.assistant_id,
                    conversation_id,
                ));
            }
            Ok(_) => {}
            Err(e) => {
                errored = true;
                error_text = "The tutor could not complete this turn.".to_string();
                tracing::warn!(error = %e, "tutor turn stream error");
                break;
            }
        }
    }

    annotate_and_finish(ctx, TurnOutcome { accumulated, errored, error_text }, emit).await;
}

/// Emits `concept_annotations` (success + non-empty text only; failures
/// degrade to no frame) followed by terminal `final`.
async fn annotate_and_finish(ctx: TurnContext, outcome: TurnOutcome, emit: impl Fn(Value)) {
    let TurnContext {
        pool,
        conversation,
        user_msg,
        request_parent,
        model,
        assistant_id,
        llm,
        title_model,
        synthesize_title,
    } = ctx;
    let conversation_id = conversation.id;
    let TurnOutcome { accumulated, errored, error_text } = outcome;

    if !errored && !accumulated.trim().is_empty() {
        match application::conversation_service::ensure_default_learner(&pool).await {
            Ok(learner_id) => {
                let graph = application::graph_service::GraphService::new(std::sync::Arc::new(
                    pool.clone(),
                ));
                match graph.annotate_turn(learner_id, &accumulated, None).await {
                    Ok(annotations) if !annotations.is_empty() => {
                        emit(annotations_frame(&annotations, assistant_id, conversation_id));
                    }
                    Ok(_) => {}
                    Err(e) => tracing::warn!(error = %e, "concept annotation failed"),
                }
            }
            Err(e) => tracing::warn!(error = %e, "concept annotation learner lookup failed"),
        }
    }

    // Reload the conversation so `updatedAt`/`title` reflect the turn.
    let final_conv = application::conversation_service::get_conversation(&pool, conversation_id)
        .await
        .ok()
        .flatten()
        .unwrap_or(conversation);

    let (response_text, error_flag) = if errored {
        if accumulated.trim().is_empty() {
            (error_text, true)
        } else {
            (accumulated.clone(), true)
        }
    } else {
        (accumulated.clone(), false)
    };

    emit(final_frame(
        &user_msg,
        &request_parent,
        model.as_deref(),
        &final_conv,
        assistant_id,
        conversation_id,
        &response_text,
        error_flag,
        chrono::Utc::now(),
    ));

    // F20: first turn only, after `final` so synthesis never delays the
    // stream. Failures keep the provisional truncation (logged in service).
    if !errored && !accumulated.trim().is_empty() && synthesize_title {
        let pool = pool.clone();
        let user_text = user_msg.content.clone();
        let assistant_text = accumulated.clone();
        tokio::spawn(async move {
            application::conversation_service::synthesize_and_store_title(
                &pool,
                &llm,
                &title_model,
                conversation_id,
                &user_text,
                &assistant_text,
            )
            .await;
        });
    }
}

/// M3 per-turn dispatch: resolve the provider for the requested model.
/// Missing credentials are an explicit 400 (never a silent wrong-provider
/// fallback); absent/unknown models resolve to the boot default.
fn resolve_turn_llm(
    model: &Option<String>,
    api_key: Option<&str>,
    state: &AppState,
) -> Result<(std::sync::Arc<dyn llm::LlmClient>, String), AppError> {
    let plan = application::llm_dispatch::plan_for(
        model.as_deref(),
        api_key,
        &application::llm_dispatch::ProviderKeys::from_env(),
    )
    .map_err(AppError::Validation)?;
    let name = plan.model.clone();
    Ok((application::llm_dispatch::build_client(&plan, state.llm.clone()), name))
}

pub async fn handle(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(_endpoint): Path<String>,
    Json(payload): Json<ChatPayload>,
) -> Result<Response, AppError> {
    let text = payload.text.clone().unwrap_or_default();
    if text.trim().is_empty() {
        return Err(AppError::Validation("text is required".into()));
    }

    let pool = pool(&state).await?.clone();

    // Validate credentials before touching the database: resolving the
    // provider is pure, so a missing key 400s here without persisting an
    // empty conversation shell.
    let (turn_llm, title_model) =
        resolve_turn_llm(&payload.model, payload.api_key.as_deref(), &state)?;

    let conversation = resolve_conversation(&pool, payload.conversation_id.as_deref()).await?;

    let model = payload.model.clone();

    // Give a brand-new conversation a provisional title immediately so the
    // sidebar and document title are meaningful before `/gen_title` runs.
    // F20: the empty-title check doubles as the first-turn detector — only
    // this turn will synthesize a title over the provisional one.
    let synthesize_title = conversation.title.as_deref().map_or(true, |t| t.trim().is_empty());
    if synthesize_title {
        let title = derive_title(&text);
        let _ = application::conversation_service::update_conversation_title(
            &pool,
            conversation.id,
            &title,
        )
        .await;
    }

    let parent_id = payload
        .parent_message_id
        .clone()
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| NO_PARENT.to_string());

    let (user_msg, assistant_id, rx) = application::tutor_service::begin_tutor_turn(
        pool.clone(),
        conversation.id,
        text,
        turn_llm.clone(),
        payload.model.clone(),
    )
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    if is_v2_start(&headers, &payload) {
        return start_v2_turn(
            state.streams.clone(),
            TurnContext {
                pool,
                conversation,
                user_msg,
                request_parent: parent_id,
                model,
                assistant_id,
                llm: turn_llm,
                title_model,
                synthesize_title,
            },
            rx,
        )
        .await;
    }

    let stream = async_stream::stream! {
        // `emit` cannot borrow across yields; route frames through a channel.
        let (tx, mut rx_out) = tokio::sync::mpsc::unbounded_channel::<Value>();
        let driver = tokio::spawn(drive_turn(
            TurnContext {
                pool,
                conversation,
                user_msg,
                request_parent: parent_id,
                model,
                assistant_id,
                llm: turn_llm,
                title_model,
                synthesize_title,
            },
            rx,
            move |frame| {
                let _ = tx.send(frame);
            },
        ));
        while let Some(frame) = rx_out.recv().await {
            let is_final = frame.get("final").is_some();
            yield Ok::<_, Infallible>(sse_data(&frame));
            if is_final {
                break;
            }
        }
        let _ = driver.await;
    };

    Ok(Sse::new(stream)
        .keep_alive(
            KeepAlive::new().interval(std::time::Duration::from_secs(15)).text("keep-alive"),
        )
        .into_response())
}

/// v2 start handshake: the turn runs in the background under `stream_id`;
/// the client attaches via [`handle_stream`]. Returns immediately so the
/// first byte is never gated on LLM latency.
async fn start_v2_turn(
    registry: StreamRegistry,
    ctx: TurnContext,
    rx: tokio::sync::mpsc::Receiver<Result<domain::TutorEvent, anyhow::Error>>,
) -> Result<Response, AppError> {
    let conversation_id = ctx.conversation.id;
    let assistant_id = ctx.assistant_id;
    let stream_id = Uuid::new_v4().to_string();
    let entry =
        new_registry_entry(registry.clone(), stream_id.clone(), conversation_id, assistant_id);
    let generation_created_at = entry.generation_created_at;

    tokio::spawn(drive_turn(ctx, rx, move |frame| entry.push(frame)));

    Ok(Json(json!({
        "generationProtocolVersion": 2,
        "status": "stream",
        "streamId": stream_id,
        "conversationId": conversation_id.to_string(),
        "generationCreatedAt": generation_created_at,
    }))
    .into_response())
}

/// `GET /api/agents/chat/stream/:stream_id` — attach to a v2 turn.
///
/// Replays the snapshot (created / latest delta / annotations / final) and
/// then forwards live frames. A lagging consumer resyncs from the snapshot
/// instead of stalling. Unknown ids 404; the client's reconnect ladder treats
/// that as terminal for the attachment.
pub async fn handle_stream(
    State(state): State<AppState>,
    Path(stream_id): Path<String>,
    Query(params): Query<StreamQuery>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    let entry = state
        .streams
        .lock()
        .map_err(|_| AppError::Internal("stream registry unavailable".into()))?
        .get(&stream_id)
        .cloned()
        .ok_or_else(|| AppError::NotFound("stream not found".into()))?;

    // The client's epoch fence (`generationCreatedAt`) disambiguates
    // replacement generations; this adapter runs one generation per stream,
    // so a mismatch only logs. `resume` needs no special casing: the snapshot
    // prefix below converges both fresh and reattaching consumers.
    tracing::debug!(
        stream_id = %stream_id,
        resume = params.resume.unwrap_or(false),
        done = entry.is_done(),
        "stream attach",
    );
    if let Some(fence) = params.generation_created_at {
        if fence != entry.generation_created_at {
            tracing::debug!(
                stream_id = %stream_id,
                fence,
                actual = entry.generation_created_at,
                "stream attach with stale generation fence",
            );
        }
    }

    let mut seen = entry.snapshot_frames();
    let prefix = seen.clone();
    let mut rx = entry.subscribe();

    let stream = async_stream::stream! {
        for frame in &prefix {
            yield Ok(sse_data(frame));
        }
        // Frames pushed between the snapshot and the subscribe land in both
        // `seen` (via a fresh snapshot) and the live channel: emit the delta
        // once by JSON comparison, then stream live.
        for frame in entry.snapshot_frames() {
            if !seen.contains(&frame) {
                seen.push(frame.clone());
                yield Ok(sse_data(&frame));
            }
        }
        if entry.is_done() {
            return;
        }
        loop {
            match rx.recv().await {
                Ok(frame) => {
                    if seen.contains(&frame) {
                        continue;
                    }
                    seen.push(frame.clone());
                    let is_final = frame.get("final").is_some();
                    yield Ok(sse_data(&frame));
                    if is_final {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    for frame in entry.snapshot_frames() {
                        if !seen.contains(&frame) {
                            seen.push(frame.clone());
                            yield Ok(sse_data(&frame));
                        }
                    }
                    if entry.is_done() {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new().interval(std::time::Duration::from_secs(15)).text("keep-alive"),
    ))
}

/// `GET /api/agents/chat/status/:conversation_id` — v2 generation status.
///
/// The client polls this to authorize terminal teardown after `final` (and
/// on foreground reattach): teardown proceeds only when the response echoes
/// `generationProtocolVersion: 2` with boolean `active: false`. While a
/// tracked turn runs, `active: true` with its epoch fences replacements.
/// Conversations with no tracked turn report `complete` (nothing running);
/// unknown conversations 404.
pub async fn handle_status(
    State(state): State<AppState>,
    Path(conversation_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let conversation_id = Uuid::parse_str(conversation_id.trim())
        .map_err(|_| AppError::Validation("conversation_id must be a UUID".into()))?;

    if let Some(entry) = latest_for_conversation(&state.streams, conversation_id) {
        let done = entry.is_done();
        return Ok(Json(json!({
            "generationProtocolVersion": 2,
            "active": !done,
            "status": if done { "complete" } else { "running" },
            "streamId": entry.stream_id,
            "createdAt": entry.generation_created_at,
            "elapsedMs": (chrono::Utc::now().timestamp_millis() - entry.generation_created_at).max(0),
        })));
    }

    let pool = pool(&state).await?;
    let known = application::conversation_service::get_conversation(pool, conversation_id)
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("database error: {e}")))?
        .is_some();
    if !known {
        return Err(AppError::NotFound("conversation not found".into()));
    }
    Ok(Json(json!({
        "generationProtocolVersion": 2,
        "active": false,
        "status": "complete",
    })))
}
