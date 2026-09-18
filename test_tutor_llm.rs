use llm::{LlmChatRequest, LlmClient, ChatMessage};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY must be set");
    let client = llm::GeminiOpenAiClient::new(key);
    
    let req = LlmChatRequest {
        model: "gemini-3.5-flash".into(),
        messages: vec![ChatMessage {
            role: "user".into(),
            content: "What is a prime number?".into(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }],
        tools: vec![],
        stream: false,
    };

    println!("Sending first request...");
    let res = client.chat(req).await?;
    println!("--- Response 1 ---\n{}", res.content);

    let req2 = LlmChatRequest {
        model: "gemini-3.5-flash".into(),
        messages: vec![
            ChatMessage { role: "user".into(), content: "What is a prime number?".into(), name: None, tool_calls: None, tool_call_id: None },
            ChatMessage { role: "assistant".into(), content: res.content.clone(), name: None, tool_calls: None, tool_call_id: None },
            ChatMessage { role: "user".into(), content: "And what is the maximum prime number less than 10?".into(), name: None, tool_calls: None, tool_call_id: None },
        ],
        tools: vec![],
        stream: false,
    };

    println!("\nSending second request...");
    let res2 = client.chat(req2).await?;
    println!("--- Response 2 ---\n{}", res2.content);

    Ok(())
}
