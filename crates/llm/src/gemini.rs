use crate::{
    error::LlmError,
    types::{
        ChatMessage, FunctionCallChunk, LlmChatRequest, LlmStream, LlmStreamChunk,
        LlmStructuredRequest, ToolCallChunk,
    },
    LlmClient,
};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

pub struct GeminiOpenAiClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl GeminiOpenAiClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: Client::builder().use_rustls_tls().build().unwrap(),
            api_key: api_key.into(),
            // Gemini OpenAI-compatible base URL
            base_url: "https://generativelanguage.googleapis.com/v1beta/openai".to_string(),
        }
    }
}

#[async_trait]
impl LlmClient for GeminiOpenAiClient {
    async fn stream_chat(&self, request: LlmChatRequest) -> Result<LlmStream, LlmError> {
        let mut messages = Vec::new();
        for m in request.messages {
            let mut payload = match &m {
                ChatMessage::System { content } => json!({"role": "system", "content": content}),
                ChatMessage::User { content } => json!({"role": "user", "content": content}),
                ChatMessage::Assistant { content, tool_calls, metadata } => {
                    let mut p = json!({"role": "assistant", "content": if content.is_empty() { serde_json::Value::Null } else { json!(content) }});
                    if let Some(meta) = metadata {
                        p["thought_signature"] = meta
                            .get("thought_signature")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null);
                    }
                    if let Some(tool_calls) = tool_calls {
                        let tc_val: Vec<serde_json::Value> = tool_calls
                            .iter()
                            .map(|tc| {
                                let mut tool_call_obj = json!({
                                    "id": tc.id,
                                    "type": "function",
                                    "function": {
                                        "name": tc.name,
                                        "arguments": tc.arguments.to_string(),
                                    },
                                });
                                if let Some(ref ts) = tc.thought_signature {
                                    tool_call_obj["function"]["thought_signature"] = json!(ts);
                                }
                                tool_call_obj
                            })
                            .collect();
                        p["tool_calls"] = json!(tc_val);
                    }
                    p
                }
                ChatMessage::Tool { content, tool_call_id, name } => json!({
                    "role": "tool",
                    "content": content,
                    "tool_call_id": tool_call_id,
                    "name": name,
                }),
            };
            messages.push(payload);
        }

        let mut payload = json!({
            "model": request.model,
            "messages": messages,
            "stream": true,
        });

        if !request.tools.is_empty() {
            let tools_val: Vec<serde_json::Value> = request
                .tools
                .into_iter()
                .map(|t| {
                    json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters,
                        }
                    })
                })
                .collect();
            payload["tools"] = json!(tools_val);
        }

        // Gemini's OpenAI-compatible endpoint accepts standard Bearer Auth BUT you must use Bearer Auth.
        // Google returned: "Missing or invalid Authorization header" when we did NOT provide Bearer Auth.
        // So passing ?key= in the URL is NOT sufficient for their OpenAI-compatible endpoint. We MUST provide Bearer auth!
        let url = format!("{}/chat/completions", self.base_url);
        tracing::debug!(payload = %serde_json::to_string_pretty(&payload).unwrap(), "Sending payload to Gemini");
        let res =
            self.client.post(&url).bearer_auth(&self.api_key).json(&payload).send().await.map_err(
                |e| LlmError::Provider { message: e.to_string(), source: Some(Box::new(e)) },
            )?;

        if !res.status().is_success() {
            let err_text = res.text().await.unwrap_or_default();
            return Err(LlmError::Provider {
                message: format!("Gemini/OpenAI error status {}: {}", err_text, err_text),
                source: None,
            });
        }

        let mut stream = res.bytes_stream();
        let (tx, rx) = mpsc::channel(64);

        tokio::spawn(async move {
            let mut buffer = String::new();

            while let Some(chunk_res) = stream.next().await {
                match chunk_res {
                    Ok(bytes) => {
                        if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                            buffer.push_str(&text);
                            while let Some(line_end) = buffer.find('\n') {
                                let line = buffer.drain(..=line_end).collect::<String>();
                                let line = line.trim();
                                if line.is_empty() {
                                    continue;
                                }
                                if line == "data: [DONE]" {
                                    return;
                                }
                                if let Some(data) = line.strip_prefix("data:") {
                                    let data = data.trim();
                                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(data)
                                    {
                                        if let Some(choices) = val.get("choices") {
                                            if let Some(choice) = choices.get(0) {
                                                if let Some(delta) = choice.get("delta") {
                                                    let content = delta
                                                        .get("content")
                                                        .and_then(|c| c.as_str())
                                                        .map(|s| s.to_string());
                                                    let mut tool_calls = None;
                                                    let mut metadata = None;
                                                    if let Some(ts) = delta.get("thought_signature")
                                                    {
                                                        metadata = Some(
                                                            json!({"thought_signature": ts.clone()}),
                                                        );
                                                    }
                                                    if let Some(tc_list) = delta
                                                        .get("tool_calls")
                                                        .and_then(|v| v.as_array())
                                                    {
                                                        let mut converted = Vec::new();
                                                        for tc in tc_list {
                                                            let index = tc
                                                                .get("index")
                                                                .and_then(|v| v.as_i64())
                                                                .unwrap_or(0)
                                                                as i32;
                                                            let id = tc
                                                                .get("id")
                                                                .and_then(|v| v.as_str())
                                                                .map(|s| s.to_string());
                                                            let mut function = None;
                                                            if let Some(func) = tc.get("function") {
                                                                let name = func
                                                                    .get("name")
                                                                    .and_then(|v| v.as_str())
                                                                    .map(|s| s.to_string());
                                                                let arguments = func
                                                                    .get("arguments")
                                                                    .and_then(|v| v.as_str())
                                                                    .map(|s| s.to_string());
                                                                // Extract thought_signature from extra_content.google.thought_signature
                                                                let thought_signature = tc
                                                                    .get("extra_content")
                                                                    .and_then(|ec| ec.get("google"))
                                                                    .and_then(|g| {
                                                                        g.get("thought_signature")
                                                                    })
                                                                    .and_then(|ts| ts.as_str())
                                                                    .map(|s| s.to_string());
                                                                function =
                                                                    Some(FunctionCallChunk {
                                                                        name,
                                                                        arguments,
                                                                        thought_signature,
                                                                    });
                                                            }
                                                            converted.push(ToolCallChunk {
                                                                index,
                                                                id,
                                                                function,
                                                            });
                                                        }
                                                        tool_calls = Some(converted);
                                                    }
                                                    let _ = tx
                                                        .send(Ok(LlmStreamChunk {
                                                            content,
                                                            tool_calls,
                                                            metadata,
                                                        }))
                                                        .await;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Err(LlmError::Provider {
                                message: e.to_string(),
                                source: Some(Box::new(e)),
                            }))
                            .await;
                        return;
                    }
                }
            }
        });

        Ok(rx)
    }

    async fn generate_structured(
        &self,
        request: LlmStructuredRequest,
    ) -> Result<serde_json::Value, LlmError> {
        let url = format!("{}/chat/completions", self.base_url);
        crate::structured::structured_completion(
            &self.client,
            &url,
            &self.api_key,
            &request.model,
            &request.prompt,
            &request.schema,
        )
        .await
    }
}
