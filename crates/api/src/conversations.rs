use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::routes::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateConversationRequest {
    pub title: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ConversationResponse {
    pub id: Uuid,
    pub learner_id: Uuid,
    pub title: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct ListConversationsResponse {
    pub conversations: Vec<ConversationResponse>,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ListMessagesResponse {
    pub messages: Vec<MessageResponse>,
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
}

fn to_conv_resp(c: domain::Conversation) -> ConversationResponse {
    ConversationResponse {
        id: c.id,
        learner_id: c.learner_id,
        title: c.title,
        created_at: c.created_at.to_rfc3339(),
        updated_at: c.updated_at.to_rfc3339(),
    }
}

fn to_msg_resp(m: domain::ConversationMessage) -> MessageResponse {
    MessageResponse {
        id: m.id,
        conversation_id: m.conversation_id,
        role: m.role.to_string(),
        content: m.content,
        created_at: m.created_at.to_rfc3339(),
    }
}

pub async fn create_conversation(
    State(state): State<AppState>,
    Json(req): Json<CreateConversationRequest>,
) -> Result<(StatusCode, Json<ConversationResponse>), crate::error::AppError> {
    let pool =
        state.pool.as_ref().ok_or_else(|| crate::error::AppError::Internal("no db pool".into()))?;
    let conv = application::conversation_service::create_conversation(pool, req.title.as_deref())
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;
    Ok((StatusCode::CREATED, Json(to_conv_resp(conv))))
}

pub async fn list_conversations(
    State(state): State<AppState>,
) -> Result<Json<ListConversationsResponse>, crate::error::AppError> {
    let pool =
        state.pool.as_ref().ok_or_else(|| crate::error::AppError::Internal("no db pool".into()))?;
    let convs = application::conversation_service::list_conversations(pool)
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;
    Ok(Json(ListConversationsResponse {
        conversations: convs.into_iter().map(to_conv_resp).collect(),
    }))
}

pub async fn get_conversation(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ConversationResponse>, crate::error::AppError> {
    let pool =
        state.pool.as_ref().ok_or_else(|| crate::error::AppError::Internal("no db pool".into()))?;
    let conv = application::conversation_service::get_conversation(pool, id)
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
        .ok_or_else(|| crate::error::AppError::NotFound("conversation not found".into()))?;
    Ok(Json(to_conv_resp(conv)))
}

pub async fn list_messages(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ListMessagesResponse>, crate::error::AppError> {
    let pool =
        state.pool.as_ref().ok_or_else(|| crate::error::AppError::Internal("no db pool".into()))?;
    // ensure conversation exists
    let conv = application::conversation_service::get_conversation(pool, id)
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;
    if conv.is_none() {
        return Err(crate::error::AppError::NotFound("conversation not found".into()));
    }
    let msgs = application::conversation_service::list_messages(pool, id)
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;
    Ok(Json(ListMessagesResponse { messages: msgs.into_iter().map(to_msg_resp).collect() }))
}

// SSE streaming for POST /v1/conversations/:id/messages
pub async fn send_message(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<SendMessageRequest>,
) -> Result<
    axum::response::Sse<
        impl tokio_stream::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>,
    >,
    crate::error::AppError,
> {
    if req.content.trim().is_empty() {
        return Err(crate::error::AppError::Validation("content is required".into()));
    }
    let pool = state
        .pool
        .as_ref()
        .ok_or_else(|| crate::error::AppError::Internal("no db pool".into()))?
        .clone();
    let llm = state.llm.clone();

    // Validate conversation exists before streaming
    let conv = application::conversation_service::get_conversation(&pool, id)
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?
        .ok_or_else(|| crate::error::AppError::NotFound("conversation not found".into()))?;
    let _ = conv;

    let mut rx = application::tutor_service::stream_tutor_turn(pool, id, req.content, llm)
        .await
        .map_err(|e| crate::error::AppError::Internal(e.to_string()))?;

    let stream = async_stream::stream! {
        while let Some(ev_res) = rx.recv().await {
            match ev_res {
                Ok(ev) => {
                    let envelope = ev.into_envelope();
                    let json = serde_json::to_string(&envelope).unwrap_or_else(|_| "{}".into());
                    let event = axum::response::sse::Event::default()
                        .event(envelope.event.clone())
                        .data(json);
                    yield Ok(event);
                }
                Err(e) => {
                    let envelope = domain::SseEnvelope::new("error", serde_json::json!({"code":"internal","message": e.to_string()}));
                    let json = serde_json::to_string(&envelope).unwrap_or_else(|_| "{}".into());
                    let event = axum::response::sse::Event::default().event("error").data(json);
                    yield Ok(event);
                    break;
                }
            }
        }
    };

    Ok(axum::response::Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keep-alive"),
    ))
}
