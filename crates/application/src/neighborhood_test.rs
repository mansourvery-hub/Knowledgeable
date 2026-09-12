//! Brick G1 tests: application neighborhood service.
//!
//! Covers: root-missing -> None (API maps to 404), bounded depth passthrough,
//! learner-confidence attachment including None for unseen concepts.

#[cfg(test)]
mod tests {
    use domain::{ConceptNode, ConceptStatus};
    use infrastructure::graph_repo;
    use sqlx::SqlitePool;
    use uuid::Uuid;

    fn node(name: &str) -> ConceptNode {
        ConceptNode {
            id: Uuid::new_v4(),
            canonical_name: name.to_string(),
            canonical_statement: format!("{name} statement."),
            learner_statement: None,
            world_confidence: 1.0,
            status: ConceptStatus::Active,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    async fn seed(pool: &SqlitePool) -> (Uuid, Uuid, Uuid) {
        let learner = crate::conversation_service::ensure_default_learner(pool).await.unwrap();
        let a = node("N-A");
        let b = node("N-B");
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
        sqlx::query(
            "INSERT INTO learner_concept_states (learner_id, concept_id, learner_confidence) VALUES (?, ?, ?)",
        )
        .bind(learner.to_string())
        .bind(b.id.to_string())
        .bind(0.2f32)
        .execute(pool)
        .await
        .unwrap();
        (learner, a.id, b.id)
    }

    #[tokio::test]
    async fn missing_root_returns_none_for_404_mapping() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        let learner = crate::conversation_service::ensure_default_learner(&pool).await.unwrap();
        let svc = crate::graph_service::GraphService::new(std::sync::Arc::new(pool));
        let out = svc.get_neighborhood(Uuid::new_v4(), learner, Some(2), Some(50)).await.unwrap();
        assert!(out.is_none());
    }

    #[tokio::test]
    async fn returns_nodes_edges_with_confidence_and_health() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        let (learner, a, b) = seed(&pool).await;
        let svc = crate::graph_service::GraphService::new(std::sync::Arc::new(pool));

        let out = svc
            .get_neighborhood(a, learner, Some(2), Some(50))
            .await
            .unwrap()
            .expect("neighborhood must exist");
        assert_eq!(out.nodes.len(), 2);
        assert_eq!(out.edges.len(), 1);

        let b_node = out.nodes.iter().find(|n| n.concept.id == b).unwrap();
        assert_eq!(b_node.learner_confidence, Some(0.2));
        assert_eq!(b_node.is_healthy, Some(false));
        assert_eq!(b_node.is_review_eligible, Some(true));

        let a_node = out.nodes.iter().find(|n| n.concept.id == a).unwrap();
        assert_eq!(a_node.learner_confidence, None);
        assert_eq!(a_node.is_healthy, None);
    }

    #[tokio::test]
    async fn depth_zero_returns_only_root() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        let (learner, a, _) = seed(&pool).await;
        let svc = crate::graph_service::GraphService::new(std::sync::Arc::new(pool));
        let out = svc.get_neighborhood(a, learner, Some(0), Some(50)).await.unwrap().unwrap();
        assert_eq!(out.nodes.len(), 1);
        assert!(out.edges.is_empty());
    }
}
