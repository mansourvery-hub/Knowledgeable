//! M8/G2 tests: wiki generation, cache, staleness, readiness gate.

#[cfg(test)]
mod tests {
    use domain::{ConceptNode, ConceptStatus};
    use sqlx::SqlitePool;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use uuid::Uuid;

    struct CannedWikiLlm {
        calls: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl llm::LlmClient for CannedWikiLlm {
        async fn stream_chat(
            &self,
            _request: llm::LlmChatRequest,
        ) -> Result<llm::LlmStream, llm::LlmError> {
            Err(llm::LlmError::Internal("unused".into()))
        }

        async fn generate_structured(
            &self,
            _request: llm::LlmStructuredRequest,
        ) -> Result<serde_json::Value, llm::LlmError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(serde_json::json!({
                "title": "Prime Number",
                "summary": "Numbers with exactly two factors.",
                "personalized_content": "You know factors well, so primes click next.",
            }))
        }
    }

    fn canned() -> (Arc<CannedWikiLlm>, Arc<dyn llm::LlmClient>) {
        let inner = Arc::new(CannedWikiLlm { calls: AtomicUsize::new(0) });
        (inner.clone(), inner)
    }

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

    async fn setup() -> (SqlitePool, Uuid, Uuid) {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        let learner = crate::conversation_service::ensure_default_learner(&pool).await.unwrap();
        (pool, learner, Uuid::new_v4())
    }

    async fn seed_prime(
        pool: &SqlitePool,
        learner: Uuid,
        prime_conf: f32,
        factor_conf: f32,
    ) -> Uuid {
        let prime = node("Prime Number");
        let factor = node("Factor");
        for n in [&prime, &factor] {
            infrastructure::graph_repo::create_concept(pool, n).await.unwrap();
        }
        sqlx::query(
            "INSERT INTO concept_relations (id, from_concept_id, to_concept_id, relation_type) VALUES (?, ?, ?, 'dependency')",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(prime.id.to_string())
        .bind(factor.id.to_string())
        .execute(pool)
        .await
        .unwrap();
        for (concept, conf) in [(prime.id, prime_conf), (factor.id, factor_conf)] {
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
        prime.id
    }

    fn svc(pool: SqlitePool, llm: Arc<dyn llm::LlmClient>) -> crate::wiki_service::WikiService {
        crate::wiki_service::WikiService::new(std::sync::Arc::new(pool), llm, "test-model".into())
    }

    #[tokio::test]
    async fn generates_on_mastery_with_anchors_and_caches() {
        let (pool, learner, _) = setup().await;
        let prime = seed_prime(&pool, learner, 0.98, 0.3).await;
        let (canned, llm) = canned();

        let page = svc(pool.clone(), llm).get_wiki(learner, prime).await.unwrap();
        assert_eq!(page.title, "Prime Number");
        assert_eq!(page.version, 1);
        assert!(!page.is_stale);
        assert!((page.learner_confidence_at_generation - 0.98).abs() < 1e-6);
        assert_eq!(page.known_prerequisites.len(), 1);
        assert_eq!(page.known_prerequisites[0].name, "Factor");
        assert_eq!(canned.calls.load(Ordering::SeqCst), 1);

        // Second view serves the cache without touching the LLM.
        let again = svc(pool.clone(), llm_clone(&canned)).get_wiki(learner, prime).await.unwrap();
        assert_eq!(again.version, 1);
        assert_eq!(canned.calls.load(Ordering::SeqCst), 1);
    }

    fn llm_clone(canned: &Arc<CannedWikiLlm>) -> Arc<dyn llm::LlmClient> {
        canned.clone()
    }

    #[tokio::test]
    async fn below_mastery_without_page_is_not_ready() {
        let (pool, learner, _) = setup().await;
        let prime = seed_prime(&pool, learner, 0.3, 0.3).await;
        let (_, llm) = canned();

        let err = svc(pool, llm).get_wiki(learner, prime).await.unwrap_err();
        assert!(err.to_string().contains("not ready"), "got: {err}");
    }

    #[tokio::test]
    async fn missing_concept_is_not_found() {
        let (pool, learner, _) = setup().await;
        let (_, llm) = canned();
        let err = svc(pool, llm).get_wiki(learner, Uuid::new_v4()).await.unwrap_err();
        assert!(err.to_string().contains("not found"), "got: {err}");
    }

    #[tokio::test]
    async fn stale_old_page_regenerates_but_recent_stale_serves() {
        let (pool, learner, _) = setup().await;
        let prime = seed_prime(&pool, learner, 0.98, 0.3).await;
        let (canned, llm) = canned();
        let service = svc(pool.clone(), llm);
        service.get_wiki(learner, prime).await.unwrap();
        assert_eq!(canned.calls.load(Ordering::SeqCst), 1);

        // Mark stale with an old timestamp: must regenerate (version 2).
        sqlx::query(
            "UPDATE concept_wiki_pages SET is_stale = 1, updated_at = '2020-01-01T00:00:00Z' WHERE learner_id = ? AND concept_id = ?",
        )
        .bind(learner.to_string())
        .bind(prime.to_string())
        .execute(&pool)
        .await
        .unwrap();
        let page = svc(pool.clone(), llm_clone(&canned)).get_wiki(learner, prime).await.unwrap();
        assert_eq!(page.version, 2);
        assert!(!page.is_stale);
        assert_eq!(canned.calls.load(Ordering::SeqCst), 2);

        // Mark stale just now: served stale without regenerating.
        sqlx::query(
            "UPDATE concept_wiki_pages SET is_stale = 1 WHERE learner_id = ? AND concept_id = ?",
        )
        .bind(learner.to_string())
        .bind(prime.to_string())
        .execute(&pool)
        .await
        .unwrap();
        let page = svc(pool.clone(), llm_clone(&canned)).get_wiki(learner, prime).await.unwrap();
        assert_eq!(page.version, 2);
        assert!(page.is_stale);
        assert_eq!(canned.calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn graph_evidence_marks_pages_stale() {
        let (pool, learner, _) = setup().await;
        let prime = seed_prime(&pool, learner, 0.98, 0.3).await;
        let (canned, llm) = canned();
        svc(pool.clone(), llm).get_wiki(learner, prime).await.unwrap();
        let conv =
            crate::conversation_service::create_conversation(&pool, Some("test")).await.unwrap();

        // Observation evidence on the concept stales its page.
        let svc2 = crate::graph_service::GraphService::new(std::sync::Arc::new(pool.clone()));
        svc2.apply_observation(&domain::LearnerObservation {
            id: Uuid::new_v4(),
            learner_id: learner,
            conversation_id: conv.id,
            concept_id: Some(prime),
            observation_type: domain::ObservationType::Confusion,
            confidence_delta: Some(-0.1),
            evidence: "test".into(),
            created_at: chrono::Utc::now(),
        })
        .await
        .unwrap();
        let page =
            infrastructure::wiki_repo::get_page(&pool, learner, prime).await.unwrap().unwrap();
        assert!(page.is_stale);
        assert_eq!(canned.calls.load(Ordering::SeqCst), 1);
    }
}
