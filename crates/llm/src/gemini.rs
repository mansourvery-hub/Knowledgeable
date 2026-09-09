use crate::{
    error::LlmError,
    types::{LlmChatRequest, LlmStream, LlmStructuredRequest, LlmStreamChunk, ToolCallChunk, FunctionCallChunk},
    LlmClient,
};
use async_trait::async_trait;
use reqwest::Client;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use serde_json::json;

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
            let mut payload = json!({
                "role": m.role,
                "content": m.content,
            });
            if let Some(name) = m.name {
                payload["name"] = json!(name);
            }
            if let Some(tool_calls) = m.tool_calls {
                let tc_val: Vec<serde_json::Value> = tool_calls.into_iter().map(|tc| {
                    json!({
                        "id": tc.id,
                        "type": tc.r#type,
                        "function": {
                            "name": tc.function.name,
                            "arguments": tc.function.arguments,
                        }
                    })
                }).collect();
                payload["tool_calls"] = json!(tc_val);
            }
            if let Some(id) = m.tool_call_id {
                payload["tool_call_id"] = json!(id);
            }
            messages.push(payload);
        }

        let mut payload = json!({
            "model": request.model,
            "messages": messages,
            "stream": true,
        });

        if !request.tools.is_empty() {
            let tools_val: Vec<serde_json::Value> = request.tools.into_iter().map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    }
                })
            }).collect();
            payload["tools"] = json!(tools_val);
        }

        // Gemini expects key as a query param or in bearer auth.
        // The official OpenAI-compatibility API supports bearer token authorization using your Gemini API key.
        let res = self.client.post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|e| LlmError::Provider { message: e.to_string(), source: Some(Box::new(e)) })?;

        if !res.status().is_success() {
            let err_text = res.text().await.unwrap_or_default();
            return Err(LlmError::Provider { message: format!("Gemini/OpenAI error status {}: {}", err_text, err_text), source: None });
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
                                if let Some(data) = line.strip_prefix("data: ") {
                                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(data) {
                                        if let Some(choices) = val.get("choices") {
                                            if let Some(choice) = choices.get(0) {
                                                if let Some(delta) = choice.get("delta") {
                                                    let content = delta.get("content").and_then(|c| c.as_str()).map(|s| s.to_string());
                                                    let mut tool_calls = None;
                                                    if let Some(tc_list) = delta.get("tool_calls").and_then(|v| v.as_array()) {
                                                        let mut converted = Vec::new();
                                                        for tc in tc_list {
                                                            let index = tc.get("index").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                                                            let id = tc.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
                                                            let r#type = tc.get("type").and_then(|v| v.as_str()).map(|s| s.to_string());
                                                            let mut function = None;
                                                            if let Some(func) = tc.get("function") {
                                                                function = Some(FunctionCallChunk {
                                                                    name: func.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                                                    arguments: func.get("arguments").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                                                });
                                                            }
                                                            converted.push(ToolCallChunk {
                                                                index,
                                                                id,
                                                                r#type,
                                                                function,
                                                            });
                                                        }
                                                        tool_calls = Some(converted);
                                                    }
                                                    let _ = tx.send(Ok(LlmStreamChunk { content, tool_calls })).await;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(LlmError::Provider { message: e.to_string(), source: Some(Box::new(e)) })).await;
                        return;
                    }
                }
            }
        });

        Ok(rx)
    }

    async fn generate_structured<T: serde::de::DeserializeOwned + Send>(
        &self,
        _request: LlmStructuredRequest,
    ) -> Result<T, LlmError> {
        Err(LlmError::Internal("generate_structured not yet implemented for Gemini".into()))
    }
}
