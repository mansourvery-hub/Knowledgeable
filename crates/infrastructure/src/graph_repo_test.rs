#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use domain::{ConceptNode, ConceptStatus};
    use sqlx::SqlitePool;

    #[tokio::test]
    async fn test_create_and_get() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        // Run migrations for the memory db
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();

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

        crate::graph_repo::create_concept(&pool, &node).await.unwrap();
        let fetched = crate::graph_repo::get_concept(&pool, node.id).await.unwrap().unwrap();
        
        assert_eq!(fetched.canonical_name, "Entropy");
    }
}
