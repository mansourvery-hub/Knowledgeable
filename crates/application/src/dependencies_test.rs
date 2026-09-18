//! Brick B2 tests: `get_dependencies` service passthrough.
//!
//! Covers: direct dependency relations returned, depth 0 yields empty,
//! concepts without dependents yield empty.

#[cfg(test)]
mod tests {
    use domain::{ConceptNode, ConceptStatus, RelationType};
    use infrastructure::graph_repo;
    use sqlx::SqlitePool;
    use uuid::Uuid;

    fn node(name: &str) -> ConceptNode {
        ConceptNode {
            id: Uuid::new_v4(),
            canonical_name: name.into(),
            canonical_statement: format!("{name} statement."),
            learner_statement: None,
            world_confidence: 1.0,
            status: ConceptStatus::Active,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    async fn mem_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        pool
    }

    async fn seed(pool: &SqlitePool) -> (Uuid, Uuid) {
        let a = node("D-A");
        let b = node("D-B");
        for n in [&a, &b] {
            graph_repo::create_concept(pool, n).await.unwrap();
        }
        sqlx::query(
            "INSERT INTO concept_relations (id, from_concept_id, to_concept_id, relation_type) VALUES (?, ?, ?, 'dependency')",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(a.id.to_string())
        .bind(b.id.to_string())
        .execute(pool)
        .await
        .unwrap();
        (a.id, b.id)
    }

    #[tokio::test]
    async fn returns_direct_dependencies_and_honors_depth_zero() {
        let pool = mem_pool().await;
        let (a, b) = seed(&pool).await;
        let svc = crate::graph_service::GraphService::new(std::sync::Arc::new(pool));

        let deps = svc.get_dependencies(a, Some(1)).await.unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].from_concept_id, a);
        assert_eq!(deps[0].to_concept_id, b);
        assert_eq!(deps[0].relation_type, RelationType::Dependency);

        assert!(svc.get_dependencies(a, Some(0)).await.unwrap().is_empty());
        assert!(svc.get_dependencies(b, Some(1)).await.unwrap().is_empty());
        assert!(svc.get_dependencies(a, None).await.unwrap().len() == 1);
    }
}
