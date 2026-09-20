//! Phase 5a/W1 tests: mastered-concepts list for the wiki browser.
//!
//! Covers weakest-first ordering (confidence, then name, then id), the exact
//! mastery-threshold boundary, cap/truncation, wiki page-state mapping, and
//! the empty set.

#[cfg(test)]
mod tests {
    use domain::{ConceptNode, ConceptStatus};
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

    fn page(learner: Uuid, concept: Uuid, stale: bool) -> domain::ConceptWikiPage {
        domain::ConceptWikiPage {
            id: Uuid::new_v4(),
            learner_id: learner,
            concept_id: concept,
            title: "T".into(),
            summary: "S".into(),
            personalized_content: "C".into(),
            known_prerequisites: Vec::new(),
            related_concepts: Vec::new(),
            learner_confidence_at_generation: 0.8,
            version: 1,
            is_stale: stale,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    async fn setup() -> (SqlitePool, Uuid) {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        let learner = crate::conversation_service::ensure_default_learner(&pool).await.unwrap();
        (pool, learner)
    }

    async fn seed(pool: &SqlitePool, learner: Uuid, name: &str, confidence: f32) -> Uuid {
        let n = node(name);
        infrastructure::graph_repo::create_concept(pool, &n).await.unwrap();
        sqlx::query(
            "INSERT INTO learner_concept_states (learner_id, concept_id, learner_confidence) VALUES (?, ?, ?)",
        )
        .bind(learner.to_string())
        .bind(n.id.to_string())
        .bind(confidence)
        .execute(pool)
        .await
        .unwrap();
        n.id
    }

    fn svc(pool: SqlitePool) -> crate::graph_service::GraphService {
        crate::graph_service::GraphService::new(std::sync::Arc::new(pool))
    }

    fn names(list: &crate::graph_service::MasteredList) -> Vec<&str> {
        list.items.iter().map(|m| m.concept.canonical_name.as_str()).collect()
    }

    #[tokio::test]
    async fn weakest_first_with_name_tiebreak() {
        let (pool, learner) = setup().await;
        seed(&pool, learner, "Beta", 0.75).await;
        seed(&pool, learner, "Alpha", 0.75).await;
        seed(&pool, learner, "Gamma", 0.98).await;

        let list = svc(pool).list_mastered_concepts(learner, 200).await.unwrap();

        assert_eq!(names(&list), vec!["Alpha", "Beta", "Gamma"]);
        assert!(!list.truncated);
    }

    #[tokio::test]
    async fn threshold_boundary_is_inclusive_and_unseen_excluded() {
        let (pool, learner) = setup().await;
        seed(&pool, learner, "Edge", domain::WIKI_MASTERY_THRESHOLD).await;
        seed(&pool, learner, "Under", 0.69).await;
        let unseen = node("Unseen");
        infrastructure::graph_repo::create_concept(&pool, &unseen).await.unwrap();

        let list = svc(pool).list_mastered_concepts(learner, 200).await.unwrap();

        assert_eq!(names(&list), vec!["Edge"]);
    }

    #[tokio::test]
    async fn cap_truncates_and_reports() {
        let (pool, learner) = setup().await;
        seed(&pool, learner, "A", 0.71).await;
        seed(&pool, learner, "B", 0.72).await;
        seed(&pool, learner, "C", 0.73).await;

        let list = svc(pool.clone()).list_mastered_concepts(learner, 2).await.unwrap();
        assert_eq!(names(&list), vec!["A", "B"]);
        assert!(list.truncated);

        let list = svc(pool).list_mastered_concepts(learner, 10).await.unwrap();
        assert_eq!(names(&list), vec!["A", "B", "C"]);
        assert!(!list.truncated);
    }

    #[tokio::test]
    async fn empty_set_has_no_items_and_no_truncation() {
        let (pool, learner) = setup().await;

        let list = svc(pool).list_mastered_concepts(learner, 200).await.unwrap();

        assert!(list.items.is_empty());
        assert!(!list.truncated);
    }

    #[tokio::test]
    async fn wiki_page_state_mapping() {
        let (pool, learner) = setup().await;
        let fresh = seed(&pool, learner, "Fresh", 0.80).await;
        let stale = seed(&pool, learner, "Stale", 0.85).await;
        seed(&pool, learner, "Bare", 0.90).await;
        infrastructure::wiki_repo::save_page(&pool, &page(learner, fresh, false)).await.unwrap();
        infrastructure::wiki_repo::save_page(&pool, &page(learner, stale, false)).await.unwrap();
        infrastructure::wiki_repo::mark_stale(&pool, learner, stale).await.unwrap();

        let list = svc(pool).list_mastered_concepts(learner, 200).await.unwrap();

        assert_eq!(names(&list), vec!["Fresh", "Stale", "Bare"]);
        let stale_of: Vec<Option<bool>> = list.items.iter().map(|m| m.wiki_stale).collect();
        assert_eq!(stale_of, vec![Some(false), Some(true), None]);
    }
}
