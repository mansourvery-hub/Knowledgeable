use crate::{
    error::LlmError,
    types::{ChatMessage, LlmChatRequest, LlmStream, LlmStructuredRequest},
    LlmClient,
};
use async_trait::async_trait;
use tokio::sync::mpsc;

/// Deterministic fake LLM for Phase 1 — streams a canned tutor response without external API.
/// Robust for dev/CI; replace with real adapter when `OPENAI_API_KEY` or similar is set.
pub struct FakeLlmClient {
    pub model: String,
}

impl FakeLlmClient {
    pub fn new(model: impl Into<String>) -> Self {
        Self { model: model.into() }
    }

    fn canned_response(&self, messages: &[ChatMessage]) -> String {
        let last_user = messages
            .iter()
            .rev()
            .find(|m| m.role == "user")
            .map(|m| m.content.as_str())
            .unwrap_or("your question");

        // Basic tutor-style response for Phase 1 — no graph, just frontier-aware placeholder.
        // In Phase 1 the graph is empty, so we teach directly while preserving the prompt contract.
        format!(
            "Great question about \"{last_user}\" — let's build from what you already know.\n\n\
            I'm your Knowledgeable tutor. In this Phase 1 skeleton, I'm streaming a response \
            without external LLM calls. The full tutor will inspect your graph, find your frontier, \
            and teach step-by-step.\n\n\
            If you had asked about quantum tunneling with no prior graph, I'd first check for \
            \"energy\" and \"probability\" foundations, propose missing \"wave function\" and \
            \"potential barrier\" concepts, and grow the graph from your curiosity. For now, \
            tell me what you'd like to understand next and I'll continue in this streaming demo."
        )
    }
}

#[async_trait]
impl LlmClient for FakeLlmClient {
    async fn stream_chat(&self, request: LlmChatRequest) -> Result<LlmStream, LlmError> {
        let full = self.canned_response(&request.messages);
        let (tx, rx) = mpsc::channel(32);

        // Stream in small chunks to emulate SSE text_delta
        tokio::spawn(async move {
            let chunks: Vec<String> =
                full.chars().collect::<Vec<_>>().chunks(18).map(|c| c.iter().collect()).collect();

            for chunk in chunks {
                if tx.send(Ok(chunk)).await.is_err() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(35)).await;
            }
        });

        Ok(rx)
    }

    async fn generate_structured<T: serde::de::DeserializeOwned + Send>(
        &self,
        _request: LlmStructuredRequest,
    ) -> Result<T, LlmError> {
        Err(LlmError::Internal("structured generation not supported by FakeLlmClient".into()))
    }
}
