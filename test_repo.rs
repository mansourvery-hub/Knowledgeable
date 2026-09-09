use sqlx::SqlitePool;
use uuid::Uuid;
use domain::{ConceptNode, ConceptStatus};
use infrastructure::graph_repo;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = SqlitePool::connect("sqlite:knowledgeable.db").await?;
    let node = ConceptNode {
        id: Uuid::new_v4(),
        canonical_name: "Entropy".to_string(),
        canonical_statement: "Entropy measures disorder.".to_string(),
        learner_statement: None,
        world_confidence: 1.0,
        status: ConceptStatus::Active,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    graph_repo::create_concept(&pool, &node).await?;
    println!("Inserted!");
    
    let fetched = graph_repo::get_concept(&pool, node.id).await?;
    println!("Fetched: {:?}", fetched);
    Ok(())
}
