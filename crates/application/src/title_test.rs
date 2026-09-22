//! F20 tests: synthesized conversation titles with truncation fallback.

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    use crate::conversation_service::{
        sanitize_title, synthesize_and_store_title, title_prompt, TITLE_MAX_CHARS,
    };

    struct CannedTitleLlm {
        calls: AtomicUsize,
        draft: serde_json::Value,
    }

    #[async_trait::async_trait]
    impl llm::LlmClient for CannedTitleLlm {
        async fn stream_chat(
            &self,
            _request: llm::LlmChatRequest,
        ) -> Result<llm::LlmStream, llm::LlmError> {
            Err(llm::LlmError::Internal("unused".into()))
        }

        async fn generate_structured(
            &self,
            _request: llm::LlmStructuredRequest,
        ) -> Result<serde_json::Value, llm::LlmError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.draft.clone())
        }
    }

    struct FailingTitleLlm;

    #[async_trait::async_trait]
    impl llm::LlmClient for FailingTitleLlm {
        async fn stream_chat(
            &self,
            _request: llm::LlmChatRequest,
        ) -> Result<llm::LlmStream, llm::LlmError> {
            Err(llm::LlmError::Internal("unused".into()))
        }

        async fn generate_structured(
            &self,
            _request: llm::LlmStructuredRequest,
        ) -> Result<serde_json::Value, llm::LlmError> {
            Err(llm::LlmError::Internal("no keys".into()))
        }
    }

    async fn setup() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        pool
    }

    #[test]
    fn title_prompt_grounds_synthesis_in_the_exchange() {
        let prompt = title_prompt("explain primes?", "A prime has exactly two factors.");
        assert!(prompt.contains("explain primes?"));
        assert!(prompt.contains("exactly two factors"));
        assert!(prompt.contains("six words"));
    }

    #[test]
    fn sanitize_title_cleans_model_drafts() {
        assert_eq!(sanitize_title("Sourdough Basics"), Some("Sourdough Basics".into()));
        assert_eq!(sanitize_title("  \"**Prime Numbers**\"\n"), Some("Prime Numbers".into()));
        assert_eq!(sanitize_title("   \n "), None);
        assert_eq!(sanitize_title(""), None);
        let long = "w".repeat(TITLE_MAX_CHARS + 10);
        let cleaned = sanitize_title(&long).unwrap();
        assert_eq!(cleaned.chars().count(), TITLE_MAX_CHARS + 1);
        assert!(cleaned.ends_with('…'));
    }

    #[tokio::test]
    async fn stores_sanitized_synthesis() {
        let pool = setup().await;
        let conv = crate::conversation_service::create_conversation(&pool, None).await.unwrap();
        let inner = Arc::new(CannedTitleLlm {
            calls: AtomicUsize::new(0),
            draft: serde_json::json!({ "title": "  \"Sourdough Basics\" " }),
        });
        let llm: Arc<dyn llm::LlmClient> = inner.clone();

        let stored = synthesize_and_store_title(
            &pool,
            &llm,
            "test-model",
            conv.id,
            "explain bread?",
            "Wild yeast…",
        )
        .await;
        assert!(stored);
        assert_eq!(inner.calls.load(Ordering::SeqCst), 1);
        let conv =
            crate::conversation_service::get_conversation(&pool, conv.id).await.unwrap().unwrap();
        assert_eq!(conv.title.as_deref(), Some("Sourdough Basics"));
    }

    #[tokio::test]
    async fn llm_failure_keeps_truncation_without_writing() {
        let pool = setup().await;
        let conv = crate::conversation_service::create_conversation(&pool, Some("explain bread?"))
            .await
            .unwrap();
        let llm: Arc<dyn llm::LlmClient> = Arc::new(FailingTitleLlm);

        let stored = synthesize_and_store_title(
            &pool,
            &llm,
            "test-model",
            conv.id,
            "explain bread?",
            "Wild yeast…",
        )
        .await;
        assert!(!stored);
        let conv =
            crate::conversation_service::get_conversation(&pool, conv.id).await.unwrap().unwrap();
        assert_eq!(conv.title.as_deref(), Some("explain bread?"));
    }

    #[tokio::test]
    async fn error_turn_never_calls_the_model() {
        let pool = setup().await;
        let conv = crate::conversation_service::create_conversation(&pool, None).await.unwrap();
        let inner = Arc::new(CannedTitleLlm {
            calls: AtomicUsize::new(0),
            draft: serde_json::json!({ "title": "Nope" }),
        });
        let llm: Arc<dyn llm::LlmClient> = inner.clone();

        let stored =
            synthesize_and_store_title(&pool, &llm, "test-model", conv.id, "explain bread?", "   ")
                .await;
        assert!(!stored);
        assert_eq!(inner.calls.load(Ordering::SeqCst), 0);
    }
}
