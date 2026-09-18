use application::tutor_service::stream_tutor_turn;
use domain::TutorEvent;
use llm::GeminiOpenAiClient;
use sqlx::SqlitePool;
use std::env;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Set environment so tutor_service detects it as Gemini.
    // Keys come from the environment only (.env, gitignored) — never hardcode.
    if env::var("GEMINI_API_KEY").unwrap_or_default().trim().is_empty() {
        anyhow::bail!("GEMINI_API_KEY must be set in the environment");
    }
    env::set_var("GEMINI_MODEL", "gemini-3.1-flash-lite");

    let api_key = env::var("GEMINI_API_KEY")?;
    let llm = Arc::new(GeminiOpenAiClient::new(api_key));

    let pool = SqlitePool::connect("sqlite::memory:").await?;
    sqlx::migrate!("../../migrations").run(&pool).await?;

    let _ = application::conversation_service::ensure_default_learner(&pool).await?;
    let conv =
        application::conversation_service::create_conversation(&pool, Some("math-test")).await?;

    println!("--- Testing Engine with REAL GEMINI ---");
    let prompts =
        vec!["What is a prime number?", "Is 15 a prime number?", "What is the next prime after 7?"];

    for prompt in prompts {
        println!("\n[USER]: {}", prompt);
        let mut rx =
            stream_tutor_turn(pool.clone(), conv.id, prompt.to_string(), llm.clone(), None).await?;

        print!("[TUTOR]: ");
        while let Some(ev) = rx.recv().await {
            match ev? {
                TutorEvent::TextDelta { text } => print!("{}", text),
                TutorEvent::TurnCompleted { .. } => println!("\n[Turn Complete]"),
                _ => {}
            }
        }
    }
    Ok(())
}
