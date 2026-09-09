use domain::{MessageRole, TutorEvent};
use llm::{ChatMessage, FakeLlmClient, LlmChatRequest, LlmClient, ToolDefinition};
use sqlx::SqlitePool;
use tokio::sync::mpsc;
use uuid::Uuid;
use crate::graph_service::GraphService;

async fn execute_tool(
    graph_service: &GraphService,
    name: &str,
    arguments: &str,
) -> Result<serde_json::Value, anyhow::Error> {
    let args: serde_json::Value = serde_json::from_str(arguments)?;
    match name {
        "find_concept" => {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(8);
            let concepts = graph_service.find_concepts(query, limit).await?;
            Ok(serde_json::to_value(concepts)?)
        }
        "get_concept" => {
            let concept_id_str = args.get("concept_id").and_then(|v| v.as_str()).unwrap_or("");
            let concept_id = Uuid::parse_str(concept_id_str)?;
            let concept = graph_service.get_concept(concept_id).await?;
            Ok(serde_json::to_value(concept)?)
        }
        "get_dependencies" => {
            let concept_id_str = args.get("concept_id").and_then(|v| v.as_str()).unwrap_or("");
            let concept_id = Uuid::parse_str(concept_id_str)?;
            let deps = graph_service.get_dependencies(concept_id).await?;
            Ok(serde_json::to_value(deps)?)
        }
        "get_related_concepts" => {
            let concept_id_str = args.get("concept_id").and_then(|v| v.as_str()).unwrap_or("");
            let concept_id = Uuid::parse_str(concept_id_str)?;
            let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(16);
            let related = graph_service.get_related_concepts(concept_id, limit).await?;
            Ok(serde_json::to_value(related)?)
        }
        _ => Err(anyhow::anyhow!("unknown tool: {}", name)),
    }
}

fn get_graph_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "find_concept".into(),
            description: "Find concepts by substring matching over canonical names and statements.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "The substring to search for." },
                    "limit": { "type": "integer", "description": "Maximum concepts to return (max 8)." }
                },
                "required": ["query"]
            }),
        },
        ToolDefinition {
            name: "get_concept".into(),
            description: "Retrieve a specific concept node by its UUID.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "concept_id": { "type": "string", "description": "The concept UUID." }
                },
                "required": ["concept_id"]
            }),
        },
        ToolDefinition {
            name: "get_dependencies".into(),
            description: "Retrieve dependency prerequisites of a concept by its UUID.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "concept_id": { "type": "string", "description": "The concept UUID." }
                },
                "required": ["concept_id"]
            }),
        },
        ToolDefinition {
            name: "get_related_concepts".into(),
            description: "Retrieve semantic neighbors of a concept by its UUID.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "concept_id": { "type": "string", "description": "The concept UUID." },
                    "limit": { "type": "integer", "description": "Maximum concepts to return (max 16)." }
                },
                "required": ["concept_id"]
            }),
        },
    ]
}

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
        .map(|m| ChatMessage {
            role: m.role.to_string(),
            content: m.content.clone(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        })
        .collect();

    // Build LLM request with basic tutor prompt + history
    let system_prompt = tutor::prompts::get_system_policy();
    let mut all_messages = vec![ChatMessage {
        role: "system".into(),
        content: system_prompt,
        name: None,
        tool_calls: None,
        tool_call_id: None,
    }];
    all_messages.extend(llm_messages.clone());

    let graph_service = GraphService::new(std::sync::Arc::new(pool.clone()));

    // Channel to caller (SSE)
    let (tx, rx) = mpsc::channel::<Result<TutorEvent, anyhow::Error>>(32);
    let pool_clone = pool.clone();
    let llm_clone = llm.clone();

    tokio::spawn(async move {
        let mut messages = all_messages;
        let mut full_assistant = String::new();
        let turn_id = Uuid::new_v4();

        loop {
            let model_name = if std::env::var("GEMINI_API_KEY").is_ok() {
                std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-1.5-flash".to_string())
            } else {
                std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string())
            };
            let req = LlmChatRequest {
                model: model_name,
                messages: messages.clone(),
                tools: get_graph_tools(),
                stream: true,
            };

            let mut llm_stream = match llm_clone.stream_chat(req).await {
                Ok(s) => s,
                Err(e) => {
                    let _ = tx
                        .send(Ok(TutorEvent::Error {
                            code: "llm_error".into(),
                            message: e.to_string(),
                        }))
                        .await;
                    return;
                }
            };

            let mut current_tool_calls: std::collections::HashMap<i32, llm::ToolCallChunk> = std::collections::HashMap::new();

            while let Some(chunk_res) = llm_stream.recv().await {
                match chunk_res {
                    Ok(chunk) => {
                        if let Some(text) = chunk.content {
                            full_assistant.push_str(&text);
                            let ev = TutorEvent::TextDelta { text };
                            if tx.send(Ok(ev)).await.is_err() {
                                return;
                            }
                        }

                        if let Some(tool_chunks) = chunk.tool_calls {
                            for tc in tool_chunks {
                                let entry = current_tool_calls.entry(tc.index).or_insert_with(|| llm::ToolCallChunk {
                                    index: tc.index,
                                    id: None,
                                    r#type: None,
                                    function: None,
                                });

                                if let Some(id) = tc.id {
                                    entry.id = Some(id);
                                }
                                if let Some(t) = tc.r#type {
                                    entry.r#type = Some(t);
                                }
                                if let Some(func) = tc.function {
                                    let current_func = entry.function.get_or_insert_with(|| llm::FunctionCallChunk {
                                        name: None,
                                        arguments: None,
                                    });
                                    if let Some(name) = func.name {
                                        current_func.name = Some(name);
                                    }
                                    if let Some(args) = func.arguments {
                                        let current_args = current_func.arguments.get_or_insert_with(String::new);
                                        current_args.push_str(&args);
                                    }
                                }
                            }
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

            if current_tool_calls.is_empty() {
                // Streaming complete, no tool calls requested. Break out of recursion loop.
                break;
            }

            // Execute requested tool calls sequentially and append tool results
            let mut assistant_tool_calls = Vec::new();
            for (_, tc) in current_tool_calls {
                if let (Some(id), Some(func)) = (tc.id, tc.function) {
                    if let (Some(name), Some(args)) = (func.name, func.arguments) {
                        assistant_tool_calls.push(llm::ToolCall {
                            id: id.clone(),
                            r#type: "function".into(),
                            function: llm::FunctionCall {
                                name: name.clone(),
                                arguments: args.clone(),
                            },
                        });
                    }
                }
            }

            // Add the assistant's request for tool execution to history
            messages.push(ChatMessage {
                role: "assistant".into(),
                content: String::new(),
                name: None,
                tool_calls: Some(assistant_tool_calls.clone()),
                tool_call_id: None,
            });

            for tc in assistant_tool_calls {
                let _ = tx.send(Ok(TutorEvent::ToolCallStarted {
                    tool_name: tc.function.name.clone(),
                    call_id: tc.id.clone(),
                })).await;

                let result = match execute_tool(&graph_service, &tc.function.name, &tc.function.arguments).await {
                    Ok(val) => val.to_string(),
                    Err(e) => serde_json::json!({ "error": e.to_string() }).to_string(),
                };

                let _ = tx.send(Ok(TutorEvent::ToolCallFinished {
                    tool_name: tc.function.name.clone(),
                    call_id: tc.id.clone(),
                })).await;

                messages.push(ChatMessage {
                    role: "tool".into(),
                    content: result,
                    name: Some(tc.function.name),
                    tool_calls: None,
                    tool_call_id: Some(tc.id),
                });
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
    if let Ok(key) = std::env::var("GEMINI_API_KEY") {
        if !key.trim().is_empty() {
            let model = std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-1.5-flash".to_string());
            tracing::info!(model = %model, "initializing real Gemini OpenAI-Compatible LLM client");
            return std::sync::Arc::new(llm::GeminiOpenAiClient::new(key));
        }
    }
    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        if !key.trim().is_empty() {
            let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
            tracing::info!(model = %model, "initializing real OpenAI LLM client");
            return std::sync::Arc::new(llm::OpenAiClient::new(key));
        }
    }
    tracing::info!("initializing fake LLM client (no API keys set)");
    std::sync::Arc::new(FakeLlmClient::new("fake-tutor-1"))
}
