use llm::{ChatMessage, GeminiOpenAiClient, LlmChatRequest, LlmClient};
use tokio_stream::StreamExt; // Need this to call next() on stream

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY must be set");
    let client = GeminiOpenAiClient::new(key);

    let req = LlmChatRequest {
        model: "gemini-3.5-flash".into(),
        messages: vec![ChatMessage::User { content: "What is a prime number?".into() }],
        tools: vec![],
        stream: true,
    };

    println!("Sending first request...");
    let mut stream = client.stream_chat(req).await?;
    let mut response1 = String::new();
    while let Some(chunk) = stream.recv().await {
        if let Ok(c) = chunk {
            if let Some(text) = c.content {
                print!("{}", text);
                response1.push_str(&text);
            }
        }
    }
    println!("\n--- Response 1 received ---");

    let req2 = LlmChatRequest {
        model: "gemini-3.5-flash".into(),
        messages: vec![
            ChatMessage::User { content: "What is a prime number?".into() },
            ChatMessage::Assistant { content: response1, tool_calls: None, metadata: None },
            ChatMessage::User {
                content: "And what is the maximum prime number less than 10?".into(),
            },
        ],
        tools: vec![],
        stream: true,
    };

    println!("\nSending second request...");
    let mut stream2 = client.stream_chat(req2).await?;
    while let Some(chunk) = stream2.recv().await {
        if let Ok(c) = chunk {
            if let Some(text) = c.content {
                print!("{}", text);
            }
        }
    }
    println!("\n--- Response 2 received ---");

    Ok(())
}
