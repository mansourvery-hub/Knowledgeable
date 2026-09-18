//! Structured (JSON) generation over OpenAI-compatible chat APIs.
//!
//! Shared by provider clients: request `response_format: json_object`,
//! extract the assistant content, and parse it defensively (models often
//! wrap JSON in markdown fences despite the format hint).

use reqwest::Client;
use serde_json::Value;

/// Strip markdown fences (```json ... ``` or ``` ... ```) around JSON.
pub fn strip_code_fences(text: &str) -> &str {
    let trimmed = text.trim();
    let unmarked = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```JSON"))
        .unwrap_or(trimmed);
    let unmarked = if unmarked.as_ptr() == trimmed.as_ptr() {
        trimmed.strip_prefix("```").unwrap_or(trimmed)
    } else {
        unmarked
    };
    unmarked.strip_suffix("```").unwrap_or(unmarked).trim()
}

pub async fn structured_completion(
    client: &Client,
    url: &str,
    api_key: &str,
    model: &str,
    prompt: &str,
    schema: &Value,
) -> Result<Value, crate::error::LlmError> {
    use crate::error::LlmError;

    // `json_object` (not strict `json_schema`) for widest provider
    // compatibility; the schema travels inline as response guidance.
    let guided = format!(
        "{prompt}\n\nRespond with a single JSON object matching this schema (no prose, no fences if avoidable): {schema}"
    );
    let payload = serde_json::json!({
        "model": model,
        "messages": [{ "role": "user", "content": guided }],
        "response_format": { "type": "json_object" },
    });
    let mut request = client.post(url);
    if !api_key.trim().is_empty() {
        request = request.bearer_auth(api_key);
    }
    let res = request
        .json(&payload)
        .send()
        .await
        .map_err(|e| LlmError::Provider { message: e.to_string(), source: Some(Box::new(e)) })?;
    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err(LlmError::Provider { message: err_text.clone(), source: None });
    }
    let body: Value = res
        .json()
        .await
        .map_err(|e| LlmError::Provider { message: e.to_string(), source: None })?;
    let content = body
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .ok_or_else(|| LlmError::Provider {
            message: "structured response had no message content".into(),
            source: None,
        })?;
    serde_json::from_str(strip_code_fences(content)).map_err(|e| LlmError::Provider {
        message: format!("structured response was not JSON: {e}"),
        source: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_fences() {
        assert_eq!(strip_code_fences("{\"a\":1}"), "{\"a\":1}");
        assert_eq!(strip_code_fences("```json\n{\"a\":1}\n```"), "{\"a\":1}");
        assert_eq!(strip_code_fences("```\n{\"a\":1}\n```"), "{\"a\":1}");
        assert_eq!(strip_code_fences("  {\"a\":1}  "), "{\"a\":1}");
    }
}
