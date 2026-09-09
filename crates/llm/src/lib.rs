//! Provider-neutral LLM abstraction per `tech_stack_and_rules.md -> LLM Abstraction`.
//! Only this crate may depend on provider SDKs. Vendor types must not leak.

use async_trait::async_trait;
use serde::de::DeserializeOwned;

pub mod error;
pub mod fake;
pub mod openai;
pub mod gemini;
pub mod types;

#[cfg(test)]
mod tests;

pub use error::{LlmError, LlmErrorKind};
pub use fake::FakeLlmClient;
pub use openai::OpenAiClient;
pub use gemini::GeminiOpenAiClient;
pub use types::{ChatMessage, LlmChatRequest, LlmStream, LlmStructuredRequest, ToolDefinition, ToolCall, FunctionCall, LlmStreamChunk, ToolCallChunk, FunctionCallChunk};

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn stream_chat(&self, request: LlmChatRequest) -> Result<LlmStream, LlmError>;

    async fn generate_structured<T: DeserializeOwned + Send>(
        &self,
        request: LlmStructuredRequest,
    ) -> Result<T, LlmError>
    where
        Self: Sized;
}
