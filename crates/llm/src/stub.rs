//! Deterministic scripted LLM for orchestration tests (M4 B3/B4).
//!
//! Unlike [`crate::FakeLlmClient`], which never calls tools, this client
//! emits one scripted tool call per entry on its first `stream_chat` call
//! and echoes received tool outputs as text on subsequent calls. That makes
//! the tutor's inspect-before-teach loop (tool dispatch, result feedback,
//! final text) deterministically testable without provider keys.

use crate::{
    error::LlmError,
    types::{
        ChatMessage, FunctionCallChunk, LlmChatRequest, LlmStream, LlmStreamChunk, ToolCallChunk,
    },
    LlmClient,
};
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::mpsc;

pub struct StubLlmClient {
    first_turn_tools: Vec<(String, serde_json::Value)>,
    calls: AtomicUsize,
}

impl StubLlmClient {
    /// Script `tools` as `(name, arguments)` pairs emitted on the first turn.
    pub fn with_tools(tools: Vec<(&str, serde_json::Value)>) -> Self {
        Self {
            first_turn_tools: tools
                .into_iter()
                .map(|(name, arguments)| (name.to_string(), arguments))
                .collect(),
            calls: AtomicUsize::new(0),
        }
    }

    fn tool_chunks(&self) -> Vec<ToolCallChunk> {
        self.first_turn_tools
            .iter()
            .enumerate()
            .map(|(index, (name, arguments))| ToolCallChunk {
                index: index as i32,
                id: Some(format!("stub-call-{index}")),
                function: Some(FunctionCallChunk {
                    name: Some(name.clone()),
                    arguments: Some(arguments.to_string()),
                    thought_signature: None,
                }),
            })
            .collect()
    }

    fn echo_text(request: &LlmChatRequest) -> String {
        let seen: Vec<&str> = request
            .messages
            .iter()
            .filter_map(|m| match m {
                ChatMessage::Tool { content, .. } => Some(content.as_str()),
                _ => None,
            })
            .collect();
        if seen.is_empty() {
            "stub reply (no tools)".to_string()
        } else {
            format!("stub final; tool outputs: {}", seen.join(" | "))
        }
    }
}

#[async_trait]
impl LlmClient for StubLlmClient {
    async fn stream_chat(&self, request: LlmChatRequest) -> Result<LlmStream, LlmError> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        // Compute the scripted chunk upfront: `tokio::spawn` needs `'static`.
        let chunk = if call == 0 && !self.first_turn_tools.is_empty() {
            LlmStreamChunk { content: None, tool_calls: Some(self.tool_chunks()), metadata: None }
        } else {
            LlmStreamChunk {
                content: Some(Self::echo_text(&request)),
                tool_calls: None,
                metadata: None,
            }
        };
        let (tx, rx) = mpsc::channel(8);
        tokio::spawn(async move {
            let _ = tx.send(Ok(chunk)).await;
        });
        Ok(rx)
    }

    async fn generate_structured(
        &self,
        _request: crate::types::LlmStructuredRequest,
    ) -> Result<serde_json::Value, LlmError> {
        Err(LlmError::Internal("structured generation not supported by StubLlmClient".into()))
    }
}
