use crate::graph_service::GraphService;
use domain::{MessageRole, TutorEvent};
use llm::{ChatMessage, FakeLlmClient, LlmChatRequest, LlmClient, ToolDefinition};
use sqlx::SqlitePool;
use tokio::sync::mpsc;
use uuid::Uuid;

async fn execute_tool(
    graph_service: &GraphService,
    conversation_id: Uuid,
    name: &str,
    arguments: &str,
) -> Result<serde_json::Value, anyhow::Error> {
    let args: serde_json::Value = serde_json::from_str(arguments).map_err(|e| {
        anyhow::anyhow!(
            "tool arguments were not valid JSON ({e}); retry {name} with a single JSON object"
        )
    })?;
    if !args.is_object() {
        return Err(anyhow::anyhow!(
            "tool arguments must be a JSON object; retry {name} with a single JSON object"
        ));
    }
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
        "get_weak_dependencies" => {
            let concept_id_str = args.get("concept_id").and_then(|v| v.as_str()).unwrap_or("");
            let concept_id = Uuid::parse_str(concept_id_str)?;
            let threshold = args.get("threshold").and_then(|v| v.as_f64()).unwrap_or(0.7) as f32;
            let depth = args.get("depth").and_then(|v| v.as_u64()).unwrap_or(1) as u8;
            let weak_deps =
                graph_service.get_weak_dependencies(concept_id, threshold, depth).await?;
            Ok(serde_json::to_value(weak_deps)?)
        }
        "get_dependencies" => {
            let concept_id_str = args.get("concept_id").and_then(|v| v.as_str()).unwrap_or("");
            let concept_id = Uuid::parse_str(concept_id_str)?;
            let depth = args.get("depth").and_then(|v| v.as_u64()).map(|d| d as u8);
            let deps = graph_service.get_dependencies(concept_id, depth).await?;
            Ok(serde_json::to_value(deps)?)
        }
        "propose_concept" => {
            let canonical_name =
                args.get("canonical_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let canonical_statement =
                args.get("canonical_statement").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let learner_statement =
                args.get("learner_statement").and_then(|v| v.as_str()).map(|s| s.to_string());
            let world_confidence =
                args.get("world_confidence").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;

            let node = domain::ConceptNode {
                id: Uuid::new_v4(),
                canonical_name,
                canonical_statement,
                learner_statement,
                world_confidence,
                status: domain::ConceptStatus::Active,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            let learner_id = graph_service.ensure_learner().await?;
            graph_service.propose_concept(learner_id, conversation_id, &node).await?;
            Ok(serde_json::json!({"status": "success", "id": node.id.to_string()}))
        }
        "propose_relation" => {
            let parse_ref =
                |v: Option<&serde_json::Value>| -> Result<domain::ConceptRef, anyhow::Error> {
                    let v = v.ok_or_else(|| anyhow::anyhow!("missing relation endpoint"))?;
                    if let Some(id) = v.get("concept_id").and_then(|x| x.as_str()) {
                        Ok(domain::ConceptRef::Existing { concept_id: Uuid::parse_str(id)? })
                    } else if let Some(id) = v.get("candidate_id").and_then(|x| x.as_str()) {
                        Ok(domain::ConceptRef::Candidate { candidate_id: Uuid::parse_str(id)? })
                    } else {
                        Err(anyhow::anyhow!("endpoint needs concept_id or candidate_id"))
                    }
                };
            let from = parse_ref(args.get("from"))?;
            let to = parse_ref(args.get("to"))?;
            let relation_type =
                args.get("relation_type").and_then(|v| v.as_str()).unwrap_or("semantic");
            let reason = args.get("reason").and_then(|v| v.as_str()).unwrap_or("").to_string();

            let learner_id = graph_service.ensure_learner().await?;
            let id = graph_service
                .propose_relation(learner_id, conversation_id, from, to, relation_type, &reason)
                .await?;
            Ok(serde_json::json!({"status": "success", "id": id.to_string()}))
        }
        "get_related_concepts" => {
            let concept_id_str = args.get("concept_id").and_then(|v| v.as_str()).unwrap_or("");
            let concept_id = Uuid::parse_str(concept_id_str)?;
            let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(16);
            let related = graph_service.get_related_concepts(concept_id, limit).await?;
            Ok(serde_json::to_value(related)?)
        }
        "log_observation" => {
            // M6/E1 validation gate: concept must resolve when given (FK would
            // otherwise fail opaquely), delta must be within [-1,1] per the
            // data model, evidence is required, and the learner row is
            // ensured so candidate/state FKs never dangle.
            let concept_id = match args.get("concept_id").and_then(|v| v.as_str()) {
                None => None,
                Some(id_str) => {
                    let concept_id = Uuid::parse_str(id_str)?;
                    let known = graph_service.get_concept(concept_id).await?.is_some();
                    if !known {
                        return Err(anyhow::anyhow!("invalid concept_id: no such concept"));
                    }
                    Some(concept_id)
                }
            };

            let obs_type_str = args.get("observation_type").and_then(|v| v.as_str()).unwrap_or("");
            let observation_type = match obs_type_str {
                "understands" => domain::ObservationType::Understands,
                "confusion" => domain::ObservationType::Confusion,
                "misconception" => domain::ObservationType::Misconception,
                "recall_failure" => domain::ObservationType::RecallFailure,
                "application_failure" => domain::ObservationType::ApplicationFailure,
                "new_understanding" => domain::ObservationType::NewUnderstanding,
                _ => return Err(anyhow::anyhow!("invalid observation type: {}", obs_type_str)),
            };

            let confidence_delta =
                args.get("confidence_delta").and_then(|v| v.as_f64()).map(|f| f as f32);
            if let Some(delta) = confidence_delta {
                domain::confidence::validate_delta(delta)
                    .map_err(|e| anyhow::anyhow!("invalid confidence_delta: {e}"))?;
            }
            let evidence = args.get("evidence").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if evidence.trim().is_empty() {
                return Err(anyhow::anyhow!("evidence is required"));
            }

            let learner_id = graph_service.ensure_learner().await?;
            let observation = domain::LearnerObservation {
                id: Uuid::new_v4(),
                learner_id,
                conversation_id,
                concept_id,
                observation_type,
                confidence_delta,
                evidence,
                created_at: chrono::Utc::now(),
            };
            let applied = graph_service.apply_observation(&observation).await?;
            Ok(serde_json::json!({
                "status": "success",
                "learner_confidence": applied.learner_confidence,
                "review_eligible": applied.review_eligible,
            }))
        }
        _ => Err(anyhow::anyhow!("unknown tool: {}", name)),
    }
}

/// Parse streamed tool-call arguments leniently.
///
/// Providers differ: OpenAI sends incremental fragments while Gemini
/// re-sends the complete arguments across chunks, which naive concatenation
/// turns into `A+A`. Prefer the whole payload; fall back to the longest
/// valid JSON prefix (first complete value), which recovers the single copy.
/// Models also wrap payloads in markdown fences or prose despite the schema;
/// strip fences and try the outermost `{...}` span before giving up.
/// Total garbage stays a string so the object gate in `execute_tool` rejects
/// it with a retry hint instead of panicking.
pub(crate) fn parse_tool_arguments(raw: &str) -> serde_json::Value {
    let text = llm::structured::strip_code_fences(raw.trim());
    if let Ok(value) = serde_json::from_str(text) {
        return value;
    }
    let mut stream = serde_json::Deserializer::from_str(text).into_iter::<serde_json::Value>();
    if let Some(Ok(value)) = stream.next() {
        return value;
    }
    if let (Some(start), Some(end)) = (text.find('{'), text.rfind('}')) {
        if start < end {
            if let Ok(value) = serde_json::from_str(&text[start..=end]) {
                return value;
            }
        }
    }
    serde_json::json!(text)
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
            name: "get_weak_dependencies".into(),
            description: "Retrieve dependency prerequisites of a concept that the learner is struggling with (low confidence).".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "concept_id": { "type": "string", "description": "The concept UUID." },
                    "threshold": { "type": "number", "description": "Confidence threshold below which to report a dependency as weak (default 0.7)." },
                    "depth": { "type": "integer", "description": "How deep into the dependency tree to check (default 1)." }
                },
                "required": ["concept_id"]
            }),
        },
        ToolDefinition {
            name: "get_dependencies".into(),
            description: "List the direct dependency prerequisites of a concept (unfiltered by learner confidence; use get_weak_dependencies for repair planning).".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "concept_id": { "type": "string", "description": "The concept UUID." },
                    "depth": { "type": "integer", "description": "Traversal depth; 0 returns empty, >= 1 returns direct dependencies (default 1)." }
                },
                "required": ["concept_id"]
            }),
        },
        ToolDefinition {
            name: "log_observation".into(),
            description: "Logs an observation about the learner's understanding, confusion, or misconceptions.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "concept_id": { "type": "string", "description": "The concept UUID, if applicable." },
                    "observation_type": { "type": "string", "description": "Type of observation: understands, confusion, misconception, recall_failure, application_failure, new_understanding." },
                    "confidence_delta": { "type": "number", "description": "Estimated impact on confidence." },
                    "evidence": { "type": "string", "description": "Brief evidence for this observation." }
                },
                "required": ["observation_type", "evidence"]
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
        ToolDefinition {
            name: "propose_concept".into(),
            description: "Propose a novel concept as a candidate (never authoritative). Admission requires world_confidence >= 0.80.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "canonical_name": { "type": "string", "description": "Concise canonical name." },
                    "canonical_statement": { "type": "string", "description": "Truth-bearing defining statement." },
                    "learner_statement": { "type": "string", "description": "Learner's own phrasing, if known." },
                    "world_confidence": { "type": "number", "description": "0..1 confidence the concept is real and correctly stated; below 0.80 is rejected." }
                },
                "required": ["canonical_name", "canonical_statement", "world_confidence"]
            }),
        },
        ToolDefinition {
            name: "propose_relation".into(),
            description: "Propose a candidate relation between existing concepts or candidates. Endpoints use {concept_id} for authoritative nodes or {candidate_id} for candidates.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "from": { "type": "object", "description": "Origin endpoint: {concept_id} or {candidate_id}." },
                    "to": { "type": "object", "description": "Target endpoint: {concept_id} or {candidate_id}." },
                    "relation_type": { "type": "string", "description": "semantic or dependency." },
                    "reason": { "type": "string", "description": "Why this relation holds." }
                },
                "required": ["from", "to", "relation_type", "reason"]
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
    model: Option<String>,
) -> Result<mpsc::Receiver<Result<TutorEvent, anyhow::Error>>, anyhow::Error> {
    let (_user_msg, _assistant_id, rx) =
        begin_tutor_turn(pool, conversation_id, user_content, llm, model).await?;
    Ok(rx)
}

/// Starts a tutor turn and returns the persisted user message plus the id the
/// assistant reply will be persisted under, alongside the live event stream.
///
/// Protocol adapters (e.g. the LibreChat SSE adapter) need both ids before the
/// reply has finished streaming so they can advertise stable message ids.
pub async fn begin_tutor_turn(
    pool: SqlitePool,
    conversation_id: Uuid,
    user_content: String,
    llm: std::sync::Arc<dyn LlmClient>,
    model: Option<String>,
) -> Result<
    (domain::ConversationMessage, Uuid, mpsc::Receiver<Result<TutorEvent, anyhow::Error>>),
    anyhow::Error,
> {
    begin_tutor_turn_with_parent(pool, conversation_id, user_content, None, llm, model).await
}

/// Parent-aware variant for branching (Phase 5): the new user message
/// parents to `parent_message_id` when it names a message in the same
/// conversation; unknown parents degrade to tail-append, never error.
pub async fn begin_tutor_turn_with_parent(
    pool: SqlitePool,
    conversation_id: Uuid,
    user_content: String,
    parent_message_id: Option<Uuid>,
    llm: std::sync::Arc<dyn LlmClient>,
    model: Option<String>,
) -> Result<
    (domain::ConversationMessage, Uuid, mpsc::Receiver<Result<TutorEvent, anyhow::Error>>),
    anyhow::Error,
> {
    // Ensure conversation exists
    let learner_id = crate::conversation_service::default_learner_id();
    let conv =
        infrastructure::conversation_repo::get_conversation(&pool, learner_id, conversation_id)
            .await
            .map_err(|e| anyhow::anyhow!("db get_conversation: {e}"))?
            .ok_or_else(|| anyhow::anyhow!("conversation not found"))?;

    // Resolve parent: must belong to this conversation, otherwise tail.
    let resolved_parent = match parent_message_id {
        Some(pid) => {
            let exists =
                infrastructure::conversation_repo::get_message(&pool, conversation_id, pid)
                    .await
                    .map_err(|e| anyhow::anyhow!("db get parent message: {e}"))?
                    .is_some();
            if exists {
                Some(pid)
            } else {
                // Tail fallback: last message's id, or None for first message.
                let history =
                    infrastructure::conversation_repo::list_messages(&pool, conversation_id)
                        .await
                        .map_err(|e| anyhow::anyhow!("load history for parent fallback: {e}"))?;
                history.last().map(|m| m.id)
            }
        }
        None => {
            let history = infrastructure::conversation_repo::list_messages(&pool, conversation_id)
                .await
                .map_err(|e| anyhow::anyhow!("load history for parent fallback: {e}"))?;
            history.last().map(|m| m.id)
        }
    };

    // Persist user message with resolved parent
    let user_msg = infrastructure::conversation_repo::create_message_with_parent(
        &pool,
        conversation_id,
        MessageRole::User,
        &user_content,
        resolved_parent,
    )
    .await
    .map_err(|e| anyhow::anyhow!("persist user message: {e}"))?;

    let assistant_message_id = Uuid::new_v4();

    // Load history for context (including the just-saved user message)
    let history = infrastructure::conversation_repo::list_messages(&pool, conversation_id)
        .await
        .map_err(|e| anyhow::anyhow!("load history: {e}"))?;

    let llm_messages: Vec<ChatMessage> = history
        .iter()
        .map(|m| match m.role {
            MessageRole::User => ChatMessage::User { content: m.content.clone() },
            MessageRole::Assistant => ChatMessage::Assistant {
                content: m.content.clone(),
                tool_calls: None,
                metadata: None,
            },
            MessageRole::System => ChatMessage::System { content: m.content.clone() },
        })
        .collect();

    // Build LLM request with basic tutor prompt + history
    let system_prompt = tutor::prompts::get_system_policy();
    let mut all_messages = vec![ChatMessage::System { content: system_prompt }];

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
        // Requested model wins; fall back to the provider env default so
        // tests and unset pickers keep prior behavior.
        let requested_model = model;

        loop {
            let is_gemini = std::env::var("GEMINI_API_KEY").is_ok();
            let model_name =
                requested_model.clone().filter(|m| !m.trim().is_empty()).unwrap_or_else(|| {
                    if is_gemini {
                        std::env::var("GEMINI_MODEL")
                            .unwrap_or_else(|_| "gemini-2.5-flash-lite".to_string())
                    } else {
                        std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string())
                    }
                });
            // Disable tool definitions for Gemini until thought_signature protocol is resolved.
            let tools = get_graph_tools();
            let req = LlmChatRequest {
                model: model_name,
                messages: messages.clone(),
                tools,
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

            let mut current_tool_calls: std::collections::HashMap<
                i32,
                (llm::ToolCallChunk, Option<serde_json::Value>),
            > = std::collections::HashMap::new();
            let mut last_thought_signature: Option<String> = None;

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
                                let entry =
                                    current_tool_calls.entry(tc.index).or_insert_with(|| {
                                        (
                                            llm::ToolCallChunk {
                                                index: tc.index,
                                                id: None,
                                                function: None,
                                            },
                                            None,
                                        )
                                    });

                                if let Some(id) = tc.id {
                                    entry.0.id = Some(id);
                                }
                                if let Some(func) = tc.function {
                                    let current_func = entry.0.function.get_or_insert_with(|| {
                                        llm::FunctionCallChunk {
                                            name: None,
                                            arguments: None,
                                            thought_signature: None,
                                        }
                                    });
                                    if let Some(name) = func.name {
                                        current_func.name = Some(name);
                                    }
                                    if let Some(args) = func.arguments {
                                        let current_args =
                                            current_func.arguments.get_or_insert_with(String::new);
                                        current_args.push_str(&args);
                                    }
                                    // Extract thought_signature from the function chunk
                                    if let Some(ts) = func.thought_signature {
                                        entry.1 = Some(serde_json::json!(ts.clone()));
                                        last_thought_signature = Some(ts.clone());
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
                messages.push(ChatMessage::Assistant {
                    content: full_assistant.clone(),
                    tool_calls: None,
                    metadata: last_thought_signature
                        .map(|ts| serde_json::json!({"thought_signature": ts})),
                });
                break;
            }

            // If we have tool calls, we don't push the full_assistant text yet (it might not be complete).
            // But we must push the request for tool execution.
            let mut assistant_tool_calls = Vec::new();
            for (_, (tc, thought_sig)) in current_tool_calls {
                if let Some(id) = tc.id {
                    if let Some(func) = tc.function {
                        if let (Some(name), Some(args)) = (func.name, func.arguments) {
                            let tool_call = llm::ToolCall {
                                id: id.clone(),
                                name: name.clone(),
                                arguments: parse_tool_arguments(&args),
                                thought_signature: thought_sig
                                    .and_then(|v| v.as_str().map(|s| s.to_string())),
                            };
                            assistant_tool_calls.push(tool_call);
                        }
                    }
                }
            }

            // Add the assistant's request for tool execution to history
            messages.push(ChatMessage::Assistant {
                content: String::new(),
                tool_calls: Some(assistant_tool_calls.clone()),
                metadata: last_thought_signature
                    .take()
                    .map(|ts| serde_json::json!({"thought_signature": ts})),
            });

            for tc in assistant_tool_calls {
                tracing::info!(tool_name = %tc.name, call_id = %tc.id, "--- Executing tool ---");
                let _ = tx
                    .send(Ok(TutorEvent::ToolCallStarted {
                        tool_name: tc.name.clone(),
                        call_id: tc.id.clone(),
                    }))
                    .await;

                let result = match execute_tool(
                    &graph_service,
                    conversation_id,
                    &tc.name,
                    &tc.arguments.to_string(),
                )
                .await
                {
                    Ok(val) => {
                        tracing::info!(tool_name = %tc.name, "--- Tool execution successful ---");
                        val.to_string()
                    }
                    Err(e) => {
                        tracing::error!(tool_name = %tc.name, error = %e, "--- Tool execution failed ---");
                        serde_json::json!({ "error": e.to_string() }).to_string()
                    }
                };

                let _ = tx
                    .send(Ok(TutorEvent::ToolCallFinished {
                        tool_name: tc.name.clone(),
                        call_id: tc.id.clone(),
                    }))
                    .await;

                messages.push(ChatMessage::Tool {
                    content: result,
                    tool_call_id: tc.id.clone(),
                    name: tc.name,
                });
            }
        }

        // Persist assistant message after streaming completes (parents to the user message, forming the tree edge)
        if !full_assistant.is_empty() {
            let _ = infrastructure::conversation_repo::create_message_with_id_and_parent(
                &pool_clone,
                assistant_message_id,
                conversation_id,
                MessageRole::Assistant,
                &full_assistant,
                Some(user_msg.id),
            )
            .await;
        }

        let _ = tx.send(Ok(TutorEvent::TurnCompleted { turn_id })).await;
    });

    // Also provide a fake-client fallback for tests without real LLM env.
    // The `llm` param already is FakeLlmClient in Phase 1; real adapter will be wired via env later.
    let _ = conv; // keep until auth is added
    Ok((user_msg, assistant_message_id, rx))
}

/// Factory for Phase 1: always returns Fake. Later, will switch on env `LLM_PROVIDER`.
pub fn default_llm() -> std::sync::Arc<dyn LlmClient> {
    if let Ok(key) = std::env::var("GEMINI_API_KEY") {
        if !key.trim().is_empty() {
            let model = std::env::var("GEMINI_MODEL")
                .unwrap_or_else(|_| "gemini-2.5-flash-lite".to_string());
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

#[cfg(test)]
mod tool_arg_tests {
    use super::{execute_tool, parse_tool_arguments};

    #[test]
    fn fenced_json_recovers_object() {
        let v = parse_tool_arguments(
            "```json\n{\"observation_type\": \"confusion\", \"evidence\": \"mixed up terms\"}\n```",
        );
        assert_eq!(v.get("observation_type").and_then(|v| v.as_str()), Some("confusion"));
    }

    #[test]
    fn prose_wrapped_json_recovers_object() {
        let v = parse_tool_arguments(
            "here are the args: {\"observation_type\": \"confusion\", \"evidence\": \"mixed up terms\"} hope this helps",
        );
        assert_eq!(v.get("observation_type").and_then(|v| v.as_str()), Some("confusion"));
    }

    #[test]
    fn gemini_double_send_recovers_first_object() {
        let one = "{\"observation_type\": \"confusion\", \"evidence\": \"mixed up terms\"}";
        let v = parse_tool_arguments(&format!("{one}{one}"));
        assert_eq!(v.get("observation_type").and_then(|v| v.as_str()), Some("confusion"));
    }

    #[test]
    fn total_garbage_stays_string_without_panic() {
        // Filed F5 symptom: the model emitted non-JSON starting with 't'.
        // The parser must not panic; the object gate in `execute_tool`
        // rejects it with a retry hint instead.
        let v = parse_tool_arguments("tell me about factors please");
        assert!(v.is_string(), "garbage must stay a string, got: {v}");
    }

    async fn mem_service() -> crate::graph_service::GraphService {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        crate::graph_service::GraphService::new(std::sync::Arc::new(pool))
    }

    #[tokio::test]
    async fn malformed_json_args_rejected_with_retry_hint() {
        // Exact filed input shape: raw non-JSON starting with 't'. Used to
        // surface serde's `invalid character: found 't' at 0` to the model.
        let svc = mem_service().await;
        let err =
            execute_tool(&svc, uuid::Uuid::new_v4(), "log_observation", "tell me about factors")
                .await
                .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("single JSON object"), "must hint the retry shape, got: {msg}");
        assert!(!msg.contains("invalid character"), "must not leak raw serde text, got: {msg}");
    }

    #[tokio::test]
    async fn non_object_args_rejected_with_retry_hint() {
        // What `parse_tool_arguments` produces for total garbage, after the
        // `Value::to_string` round-trip: a JSON string, not an object.
        // Must fail with a retry hint, not a confusing field error.
        let svc = mem_service().await;
        let err = execute_tool(
            &svc,
            uuid::Uuid::new_v4(),
            "log_observation",
            "\"tell me about factors\"",
        )
        .await
        .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("must be a JSON object"), "must name the problem, got: {msg}");
    }
}
