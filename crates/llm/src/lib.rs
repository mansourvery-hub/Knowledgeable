//! Provider-neutral LLM abstraction per `tech_stack_and_rules.md -> LLM Abstraction`.
//! Only this crate may depend on provider SDKs. Vendor types must not leak.

use async_trait::async_trait;

pub mod error;
pub mod fake;
pub mod gemini;
pub mod openai;
pub mod structured;
pub mod stub;
pub mod types;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_config;

pub use error::{LlmError, LlmErrorKind};
pub use fake::FakeLlmClient;
pub use gemini::GeminiOpenAiClient;
pub use openai::OpenAiClient;
pub use stub::StubLlmClient;
pub use types::{
    ChatMessage, FunctionCallChunk, LlmChatRequest, LlmStream, LlmStreamChunk,
    LlmStructuredRequest, ToolCall, ToolCallChunk, ToolDefinition,
};

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn stream_chat(&self, request: LlmChatRequest) -> Result<LlmStream, LlmError>;

    /// Structured JSON generation. Object-safe (value in/out) so services can
    /// hold trait objects; providers without support return `Internal`.
    async fn generate_structured(
        &self,
        request: LlmStructuredRequest,
    ) -> Result<serde_json::Value, LlmError>;
}
