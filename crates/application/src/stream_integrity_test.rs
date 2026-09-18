#[cfg(test)]
mod tests {
    use anyhow::Result;
    use llm::LlmStreamChunk;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_stream_chunk_parsing_integrity() {
        let (tx, mut rx) = mpsc::channel::<Result<LlmStreamChunk>>(10);

        tx.send(Ok(LlmStreamChunk {
            content: Some("Hello".into()),
            tool_calls: None,
            metadata: None,
        }))
        .await
        .unwrap();
        tx.send(Ok(LlmStreamChunk {
            content: Some(" world".into()),
            tool_calls: None,
            metadata: None,
        }))
        .await
        .unwrap();
        drop(tx);

        let mut collected = String::new();
        while let Some(chunk) = rx.recv().await {
            let chunk = chunk.unwrap();
            if let Some(content) = chunk.content {
                collected.push_str(&content);
            }
        }

        assert_eq!(collected, "Hello world");
    }
}
