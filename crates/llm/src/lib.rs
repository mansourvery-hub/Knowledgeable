//! Provider-neutral LLM abstraction per `tech_stack_and_rules.md -> LLM Abstraction`.
//! Only this crate may depend on provider SDKs. Vendor types must not leak.

use async_trait::async_trait;
use serde::de::DeserializeOwned;

pub mod error;
pub mod types;

pub use error::{LlmError, LlmErrorKind};
pub use types::{LlmChatRequest, LlmStream, LlmStructuredRequest};

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn stream_chat(&self, request: LlmChatRequest) -> Result<LlmStream, LlmError>;

    async fn generate_structured<T: DeserializeOwned + Send>(
        &self,
        request: LlmStructuredRequest,
    ) -> Result<T, LlmError>;
}
