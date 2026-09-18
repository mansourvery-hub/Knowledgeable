use application::tutor_service::stream_tutor_turn;
use domain::TutorEvent;
use llm::GeminiOpenAiClient;
use sqlx::SqlitePool;
use std::env;
use std::sync::Arc;
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    fmt().with_env_filter(EnvFilter::from_default_env()).init();

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
        application::conversation_service::create_conversation(&pool, Some("tool-test")).await?;

    // Pre-populate concepts
    let graph = application::graph_service::GraphService::new(Arc::new(pool.clone()));

    let prime_concept = domain::ConceptNode {
        id: uuid::Uuid::new_v4(),
        canonical_name: "Prime Number".into(),
        canonical_statement: "A number with two factors".into(),
        learner_statement: None,
        world_confidence: 1.0,
        status: domain::ConceptStatus::Active,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    graph.create_concept(&prime_concept).await?;

    let composite_concept = domain::ConceptNode {
        id: uuid::Uuid::new_v4(),
        canonical_name: "Composite Number".into(),
        canonical_statement: "A number with more than two factors".into(),
        learner_statement: None,
        world_confidence: 1.0,
        status: domain::ConceptStatus::Active,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    graph.create_concept(&composite_concept).await?;

    // Weak prerequisite: Factor (confidence 0.3) that Prime Number depends on.
    let factor_concept = domain::ConceptNode {
        id: uuid::Uuid::new_v4(),
        canonical_name: "Factor".into(),
        canonical_statement: "A number that divides another number evenly.".into(),
        learner_statement: None,
        world_confidence: 1.0,
        status: domain::ConceptStatus::Active,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    graph.create_concept(&factor_concept).await?;
    sqlx::query(
        "INSERT INTO concept_relations (id, from_concept_id, to_concept_id, relation_type) VALUES (?, ?, ?, 'dependency')",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(prime_concept.id.to_string())
    .bind(factor_concept.id.to_string())
    .execute(&pool)
    .await?;
    let learner = application::conversation_service::ensure_default_learner(&pool).await?;
    graph.update_learner_confidence(learner, factor_concept.id, 0.3).await?;

    println!("--- Turn 1: Learning about Prime Numbers ---");
    let prompt1 = format!(
        "I am very confused about the concept: {} (ID: {}). I don't understand it at all.",
        prime_concept.canonical_name, prime_concept.id
    );

    let mut rx = stream_tutor_turn(pool.clone(), conv.id, prompt1, llm.clone(), None).await?;

    while let Some(ev) = rx.recv().await {
        match ev {
            Ok(TutorEvent::TextDelta { text }) => print!("{}", text),
            Ok(TutorEvent::ToolCallStarted { tool_name, call_id }) => {
                println!("\n[TOOL CALL STARTED]: {} (ID: {})", tool_name, call_id)
            }
            Ok(TutorEvent::ToolCallFinished { tool_name, call_id }) => {
                println!("\n[TOOL CALL FINISHED]: {} (ID: {})", tool_name, call_id)
            }
            Ok(TutorEvent::TurnCompleted { .. }) => println!("\n[Turn Complete]"),
            Ok(TutorEvent::Error { code, message }) => println!("\n[Error {}: {}]", code, message),
            Err(e) => println!("\n[Error: {}]", e),
            _ => {}
        }
    }

    // Turn 2: Ask about composite numbers - should know we understand primes
    println!("\n--- Turn 2: Asking about Composite Numbers (should know primes understood) ---");
    let prompt2 = format!(
        "Now teach me about {} (ID: {})",
        composite_concept.canonical_name, composite_concept.id
    );

    let mut rx2 = stream_tutor_turn(pool.clone(), conv.id, prompt2, llm.clone(), None).await?;

    while let Some(ev) = rx2.recv().await {
        match ev {
            Ok(TutorEvent::TextDelta { text }) => print!("{}", text),
            Ok(TutorEvent::ToolCallStarted { tool_name, call_id }) => {
                println!("\n[TOOL CALL STARTED]: {} (ID: {})", tool_name, call_id)
            }
            Ok(TutorEvent::ToolCallFinished { tool_name, call_id }) => {
                println!("\n[TOOL CALL FINISHED]: {} (ID: {})", tool_name, call_id)
            }
            Ok(TutorEvent::TurnCompleted { .. }) => println!("\n[Turn Complete]"),
            Ok(TutorEvent::Error { code, message }) => println!("\n[Error {}: {}]", code, message),
            Err(e) => println!("\n[Error: {}]", e),
            _ => {}
        }
    }
    Ok(())
}
