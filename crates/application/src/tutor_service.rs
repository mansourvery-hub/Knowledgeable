use domain::{MessageRole, TutorEvent};
use llm::{ChatMessage, FakeLlmClient, LlmChatRequest, LlmClient};
use sqlx::SqlitePool;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Streams a tutor turn: persists user message, calls LLM with history, streams deltas, persists assistant message.
/// For Phase 1 the LLM is the fake client and no graph is consulted — the SSE contract is preserved for future frontier teaching.
pub async fn stream_tutor_turn(
    pool: SqlitePool,
    conversation_id: Uuid,
    user_content: String,
    llm: std::sync::Arc<dyn LlmClient>,
) -> Result<mpsc::Receiver<Result<TutorEvent, anyhow::Error>>, anyhow::Error> {
    // Ensure conversation exists
    let learner_id = crate::conversation_service::default_learner_id();
    let conv =
        infrastructure::conversation_repo::get_conversation(&pool, learner_id, conversation_id)
            .await
            .map_err(|e| anyhow::anyhow!("db get_conversation: {e}"))?
            .ok_or_else(|| anyhow::anyhow!("conversation not found"))?;

    // Persist user message
    let _user_msg = infrastructure::conversation_repo::create_message(
        &pool,
        conversation_id,
        MessageRole::User,
        &user_content,
    )
    .await
    .map_err(|e| anyhow::anyhow!("persist user message: {e}"))?;

    // Load history for context (including the just-saved user message)
    let history = infrastructure::conversation_repo::list_messages(&pool, conversation_id)
        .await
        .map_err(|e| anyhow::anyhow!("load history: {e}"))?;

    let llm_messages: Vec<ChatMessage> = history
        .iter()
        .map(|m| ChatMessage { role: m.role.to_string(), content: m.content.clone() })
        .collect();

    // Build LLM request with basic tutor prompt + history
    let system_prompt = tutor::prompts::SYSTEM_POLICY.to_string();
    let mut all_messages = vec![ChatMessage { role: "system".into(), content: system_prompt }];
    all_messages.extend(llm_messages);

    let req = LlmChatRequest {
        model: "fake-tutor-1".into(),
        messages: all_messages,
        tools: vec![],
        stream: true,
    };

    let mut llm_stream =
        llm.stream_chat(req).await.map_err(|e| anyhow::anyhow!("llm stream start: {e}"))?;

    // Channel to caller (SSE)
    let (tx, rx) = mpsc::channel::<Result<TutorEvent, anyhow::Error>>(32);
    let pool_clone = pool.clone();

    tokio::spawn(async move {
        let mut full_assistant = String::new();
        let turn_id = Uuid::new_v4();

        while let Some(chunk_res) = llm_stream.recv().await {
            match chunk_res {
                Ok(text) => {
                    full_assistant.push_str(&text);
                    let ev = TutorEvent::TextDelta { text: text.clone() };
                    if tx.send(Ok(ev)).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    let _ = tx
                        .send(Ok(TutorEvent::Error {
                            code: "llm_error".into(),
                            message: e.to_string(),
                        }))
                        .await;
                    return;
                }
            }
        }

        // Persist assistant message after streaming completes
        if !full_assistant.is_empty() {
            let _ = infrastructure::conversation_repo::create_message(
                &pool_clone,
                conversation_id,
                MessageRole::Assistant,
                &full_assistant,
            )
            .await;
        }

        let _ = tx.send(Ok(TutorEvent::TurnCompleted { turn_id })).await;
    });

    // Also provide a fake-client fallback for tests without real LLM env.
    // The `llm` param already is FakeLlmClient in Phase 1; real adapter will be wired via env later.
    let _ = conv; // keep until auth is added
    Ok(rx)
}

/// Factory for Phase 1: always returns Fake. Later, will switch on env `LLM_PROVIDER`.
pub fn default_llm() -> std::sync::Arc<dyn LlmClient> {
    std::sync::Arc::new(FakeLlmClient::new("fake-tutor-1"))
}
