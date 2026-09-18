//! Brick A2 tests: turn annotation service.
//!
//! Covers: known/weak/new badge mapping, archived exclusion, subword
//! rejection, empty-text short-circuit, limit bounding.

#[cfg(test)]
mod tests {
    use domain::{AnnotationStatus, ConceptNode, ConceptStatus};
    use infrastructure::graph_repo;
    use sqlx::SqlitePool;
    use uuid::Uuid;

    fn node(name: &str, status: ConceptStatus) -> ConceptNode {
        ConceptNode {
            id: Uuid::new_v4(),
            canonical_name: name.into(),
            canonical_statement: format!("{name} statement."),
            learner_statement: None,
            world_confidence: 1.0,
            status,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    async fn seed(pool: &SqlitePool) -> Uuid {
        let learner = crate::conversation_service::ensure_default_learner(pool).await.unwrap();
        let prime = node("Prime Number", ConceptStatus::Active);
        let factor = node("Factor", ConceptStatus::Active);
        let unseen = node("Unseen Thing", ConceptStatus::Active);
        let old = node("Old Hat", ConceptStatus::Archived);
        for n in [&prime, &factor, &unseen, &old] {
            graph_repo::create_concept(pool, n).await.unwrap();
        }
        for (concept, conf) in [(prime.id, 0.98f32), (factor.id, 0.3f32)] {
            sqlx::query(
                "INSERT INTO learner_concept_states (learner_id, concept_id, learner_confidence) VALUES (?, ?, ?)",
            )
            .bind(learner.to_string())
            .bind(concept.to_string())
            .bind(conf)
            .execute(pool)
            .await
            .unwrap();
        }
        learner
    }

    fn svc(pool: SqlitePool) -> crate::graph_service::GraphService {
        crate::graph_service::GraphService::new(std::sync::Arc::new(pool))
    }

    async fn mem_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn annotates_known_weak_and_new_in_mention_order() {
        let pool = mem_pool().await;
        let learner = seed(&pool).await;
        let out = svc(pool)
            .annotate_turn(
                learner,
                "Unseen Thing aside, every Factor helps explain a prime number.",
                None,
            )
            .await
            .unwrap();
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].name, "Unseen Thing");
        assert_eq!(out[0].status, AnnotationStatus::New);
        assert_eq!(out[0].learner_confidence, None);
        assert_eq!(out[1].name, "Factor");
        assert_eq!(out[1].status, AnnotationStatus::Weak);
        assert_eq!(out[2].name, "Prime Number");
        assert_eq!(out[2].status, AnnotationStatus::Known);
    }

    #[tokio::test]
    async fn excludes_archived_and_subword_matches() {
        let pool = mem_pool().await;
        let learner = seed(&pool).await;
        // "Old Hat" is archived; "factory" must not match "Factor".
        let out = svc(pool)
            .annotate_turn(learner, "That old hat came from the factory.", None)
            .await
            .unwrap();
        assert!(out.is_empty(), "unexpected annotations: {out:?}");
    }

    #[tokio::test]
    async fn empty_text_short_circuits_and_limit_bounds() {
        let pool = mem_pool().await;
        let learner = seed(&pool).await;
        let service = svc(pool);
        assert!(service.annotate_turn(learner, "   ", None).await.unwrap().is_empty());
        let out = service.annotate_turn(learner, "Prime Number and Factor", Some(1)).await.unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "Prime Number");
    }
}
