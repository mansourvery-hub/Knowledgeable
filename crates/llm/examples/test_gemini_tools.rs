use llm::{LlmClient, LlmChatRequest, ChatMessage, ToolDefinition};
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
    println!("Testing Gemini client tools call with model: {} using key: {}...", model, &api_key[0..6]);

    let client = llm::GeminiOpenAiClient::new(api_key);
    
    // We send some concepts search tool schemas to verify if Gemini rejects them, fails, or streams them correctly.
    let tools = vec![
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
        }
    ];

    let req = LlmChatRequest {
        model,
        messages: vec![ChatMessage {
            role: "user".into(),
            content: "Please find the concept for 'gravity' using the find_concept tool.".into(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }],
        tools,
        stream: true,
    };

    println!("Starting chat stream with tools...");
    let mut rx = match client.stream_chat(req).await {
        Ok(stream) => stream,
        Err(e) => {
            println!("Failed to start stream with tools: {:?}", e);
            return Ok(());
        }
    };

    println!("Reading chunks for tool calls:");
    let mut tool_calls = Vec::new();
    while let Some(chunk_res) = rx.recv().await {
        match chunk_res {
            Ok(chunk) => {
                if let Some(text) = chunk.content {
                    print!("{}", text);
                    std::io::Write::flush(&mut std::io::stdout())?;
                }
                if let Some(tc_chunks) = chunk.tool_calls {
                    for tc in tc_chunks {
                        println!("\nReceived tool call chunk: index={}, id={:?}, func_name={:?}, args={:?}", 
                            tc.index, tc.id, tc.function.as_ref().and_then(|f| f.name.as_ref()), tc.function.as_ref().and_then(|f| f.arguments.as_ref()));
                        tool_calls.push(tc);
                    }
                }
            }
            Err(e) => {
                println!("\nError during streaming: {:?}", e);
                return Ok(());
            }
        }
    }

    println!("\n\nStream finished! Total tool call chunks received: {}", tool_calls.len());
    Ok(())
}
