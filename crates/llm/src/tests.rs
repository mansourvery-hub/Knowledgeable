#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LlmClient, LlmChatRequest, ChatMessage, LlmStreamChunk, ToolCallChunk, FunctionCallChunk};
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_mock_chat_stream() {
        let client = crate::FakeLlmClient::new("fake");
        let req = LlmChatRequest {
            model: "fake".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: "Hello".into(),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            }],
            tools: vec![],
            stream: true,
        };

        let mut rx = client.stream_chat(req).await.unwrap();
        let mut chunks = Vec::new();
        while let Some(res) = rx.recv().await {
            let chunk = res.unwrap();
            chunks.push(chunk);
        }

        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.content.is_some()));
    }
}
