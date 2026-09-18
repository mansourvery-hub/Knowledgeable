use application::tutor_service::stream_tutor_turn;
use domain::TutorEvent;
use llm::{GeminiOpenAiClient, LlmClient};
use std::env;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = env::var("GEMINI_API_KEY")?;
    let llm = Arc::new(GeminiOpenAiClient::new(api_key));

    println!("Testing Gemini direct stream...");
    let req = llm::LlmChatRequest {
        model: "gemini-3.1-flash-lite".into(),
        messages: vec![llm::ChatMessage::User { content: "Explain quantum tunneling".into() }],

        tools: vec![],
        stream: true,
    };

    match llm.stream_chat(req).await {
        Ok(mut rx) => {
            while let Some(res) = rx.recv().await {
                match res {
                    Ok(chunk) => println!("Chunk: {:?}", chunk.content),
                    Err(e) => println!("Error chunk: {:?}", e),
                }
            }
        }
        Err(e) => println!("Stream error: {:?}", e),
    }
    Ok(())
}
