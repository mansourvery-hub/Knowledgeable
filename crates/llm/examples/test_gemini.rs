use llm::{LlmClient, LlmChatRequest, ChatMessage};
use dotenvy::dotenv;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenv();
    
    let api_key = match env::var("GEMINI_API_KEY") {
        Ok(k) if !k.trim().is_empty() => k,
        _ => {
            println!("Error: GEMINI_API_KEY is not set or empty in your .env file!");
            return Ok(());
        }
    };

    let model = env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-1.5-flash".to_string());
    println!("Testing Gemini client with model: {} using key: {}...", model, &api_key[0..6]);

    let client = llm::GeminiOpenAiClient::new(api_key);
    let req = LlmChatRequest {
        model,
        messages: vec![ChatMessage {
            role: "user".into(),
            content: "Write a short 2-sentence response saying hello.".into(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }],
        tools: vec![],
        stream: true,
    };

    println!("Starting chat stream...");
    let mut rx = match client.stream_chat(req).await {
        Ok(stream) => stream,
        Err(e) => {
            println!("Failed to start chat stream: {:?}", e);
            return Ok(());
        }
    };

    println!("Stream successfully started! Reading chunks:");
    let mut full_response = String::new();
    while let Some(chunk_res) = rx.recv().await {
        match chunk_res {
            Ok(chunk) => {
                if let Some(text) = chunk.content {
                    print!("{}", text);
                    std::io::Write::flush(&mut std::io::stdout())?;
                    full_response.push_str(&text);
                }
            }
            Err(e) => {
                println!("\nError during streaming: {:?}", e);
                return Ok(());
            }
        }
    }

    println!("\n\nStream finished successfully!");
    println!("Full Response: {}", full_response);

    Ok(())
}
