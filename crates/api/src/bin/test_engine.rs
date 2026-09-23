use application::tutor_service::{default_llm, stream_tutor_turn};
use domain::TutorEvent;
use sqlx::SqlitePool;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Note: migrations path relative to crate root
    let pool = SqlitePool::connect("sqlite::memory:").await?;
    sqlx::migrate!("../../migrations").run(&pool).await?;

    let _learner_id = application::conversation_service::ensure_default_learner(&pool).await?;
    let conv = application::conversation_service::create_conversation(&pool, Some("test")).await?;
    let llm = default_llm();

    println!("--- Testing Engine: Chat Loop ---");
    let prompts = vec!["What is a prime number?", "Is 4 a prime number?", "What is the next one?"];

    for prompt in prompts {
        println!("\nUser: {}", prompt);
        let mut rx =
            stream_tutor_turn(pool.clone(), conv.id, prompt.into(), llm.clone(), None).await?;

        print!("Tutor: ");
        while let Some(ev) = rx.recv().await {
            match ev? {
                TutorEvent::TextDelta { text } => print!("{}", text),
                TutorEvent::TurnCompleted { .. } => println!(" [Turn Complete]"),
                _ => {}
            }
        }
    }
    Ok(())
}
