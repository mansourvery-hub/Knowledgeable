use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::Assistant => write!(f, "assistant"),
            Self::System => write!(f, "system"),
        }
    }
}

impl std::str::FromStr for MessageRole {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user" => Ok(Self::User),
            "assistant" => Ok(Self::Assistant),
            "system" => Ok(Self::System),
            _ => Err(format!("invalid role: {s}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: Uuid,
    pub learner_id: Uuid,
    pub title: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub role: MessageRole,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseEnvelope {
    pub version: u8,
    pub event: String,
    pub data: serde_json::Value,
}

impl SseEnvelope {
    pub fn new(event: &str, data: serde_json::Value) -> Self {
        Self { version: 1, event: event.to_string(), data }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TutorEvent {
    TextDelta { text: String },
    ToolCallStarted { tool_name: String, call_id: String },
    ToolCallFinished { tool_name: String, call_id: String },
    GraphUpdatePending,
    GraphUpdateCommitted { mutation_id: Uuid },
    ReviewSuggested { concept_ids: Vec<Uuid> },
    TurnCompleted { turn_id: Uuid },
    Error { code: String, message: String },
}

impl TutorEvent {
    pub fn event_name(&self) -> &'static str {
        match self {
            Self::TextDelta { .. } => "text_delta",
            Self::ToolCallStarted { .. } => "tool_call_started",
            Self::ToolCallFinished { .. } => "tool_call_finished",
            Self::GraphUpdatePending => "graph_update_pending",
            Self::GraphUpdateCommitted { .. } => "graph_update_committed",
            Self::ReviewSuggested { .. } => "review_suggested",
            Self::TurnCompleted { .. } => "turn_completed",
            Self::Error { .. } => "error",
        }
    }

    pub fn into_envelope(self) -> SseEnvelope {
        let event = self.event_name().to_string();
        let data = match &self {
            Self::TextDelta { text } => serde_json::json!({"text": text}),
            Self::ToolCallStarted { tool_name, call_id } => {
                serde_json::json!({"tool_name": tool_name, "call_id": call_id})
            }
            Self::ToolCallFinished { tool_name, call_id } => {
                serde_json::json!({"tool_name": tool_name, "call_id": call_id})
            }
            Self::GraphUpdatePending => serde_json::json!({}),
            Self::GraphUpdateCommitted { mutation_id } => {
                serde_json::json!({"mutation_id": mutation_id})
            }
            Self::ReviewSuggested { concept_ids } => {
                serde_json::json!({"concept_ids": concept_ids})
            }
            Self::TurnCompleted { turn_id } => serde_json::json!({"turn_id": turn_id}),
            Self::Error { code, message } => serde_json::json!({"code": code, "message": message}),
        };
        SseEnvelope { version: 1, event, data }
    }
}
