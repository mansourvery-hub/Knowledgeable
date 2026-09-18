//! M5 candidate tests: proposal gates (D1), relation repair (D2), atomic
//! admission with audit (D3).

#[cfg(test)]
mod tests {
    use domain::{ConceptNode, ConceptStatus};
    use sqlx::SqlitePool;
    use uuid::Uuid;

    fn node(name: &str, world_confidence: f32) -> ConceptNode {
        ConceptNode {
            id: Uuid::new_v4(),
            canonical_name: name.into(),
            canonical_statement: format!("{name} statement."),
            learner_statement: None,
            world_confidence,
            status: ConceptStatus::Active,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    async fn setup() -> (SqlitePool, Uuid, Uuid) {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        let learner = crate::conversation_service::ensure_default_learner(&pool).await.unwrap();
        let conv =
            crate::conversation_service::create_conversation(&pool, Some("test")).await.unwrap();
        (pool, learner, conv.id)
    }

    fn svc(pool: SqlitePool) -> crate::graph_service::GraphService {
        crate::graph_service::GraphService::new(std::sync::Arc::new(pool))
    }

    async fn candidate_count(pool: &SqlitePool) -> i64 {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM concept_candidates")
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn valid_proposal_lands_pending_with_scope() {
        let (pool, learner, conv_id) = setup().await;
        let service = svc(pool.clone());
        let n = node("Novel Concept", 0.9);

        service.propose_concept(learner, conv_id, &n).await.unwrap();

        assert_eq!(candidate_count(&pool).await, 1);
        let row: (String, String, String) = sqlx::query_as(
            "SELECT learner_id, conversation_id, status FROM concept_candidates WHERE id = ?",
        )
        .bind(n.id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.0, learner.to_string());
        assert_eq!(row.1, conv_id.to_string());
        assert_eq!(row.2, "pending");
    }

    #[tokio::test]
    async fn low_confidence_proposal_rejected_without_row() {
        let (pool, learner, conv_id) = setup().await;
        let service = svc(pool.clone());

        let err = service
            .propose_concept(learner, conv_id, &node("Shaky Concept", 0.5))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("proposal rejected"), "got: {err}");
        assert_eq!(candidate_count(&pool).await, 0);
    }

    #[tokio::test]
    async fn empty_name_proposal_rejected_without_row() {
        let (pool, learner, conv_id) = setup().await;
        let service = svc(pool.clone());
        let mut n = node("x", 0.9);
        n.canonical_name = "   ".into();

        let err = service.propose_concept(learner, conv_id, &n).await.unwrap_err();
        assert!(err.to_string().contains("proposal rejected"), "got: {err}");
        assert_eq!(candidate_count(&pool).await, 0);
    }

    async fn seed_concept(pool: &SqlitePool, name: &str) -> Uuid {
        let n = node(name, 1.0);
        let id = n.id;
        infrastructure::graph_repo::create_concept(pool, &n).await.unwrap();
        id
    }

    async fn relation_count(pool: &SqlitePool) -> i64 {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM relation_candidates")
            .fetch_one(pool)
            .await
            .unwrap()
    }

    use domain::ConceptRef;

    #[tokio::test]
    async fn relation_existing_to_existing_accepted_pending() {
        let (pool, learner, conv_id) = setup().await;
        let service = svc(pool.clone());
        let a = seed_concept(&pool, "Rel-A").await;
        let b = seed_concept(&pool, "Rel-B").await;

        let id = service
            .propose_relation(
                learner,
                conv_id,
                ConceptRef::Existing { concept_id: a },
                ConceptRef::Existing { concept_id: b },
                "dependency",
                "B grounds A",
            )
            .await
            .unwrap();

        assert_eq!(relation_count(&pool).await, 1);
        let row: (String, Option<String>, Option<String>, String, String) = sqlx::query_as(
            "SELECT status, from_concept_id, to_concept_id, relation_type, reason FROM relation_candidates WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.0, "pending");
        assert_eq!(row.1, Some(a.to_string()));
        assert_eq!(row.2, Some(b.to_string()));
        assert_eq!(row.3, "dependency");
        assert_eq!(row.4, "B grounds A");
    }

    #[tokio::test]
    async fn relation_to_candidate_accepted() {
        let (pool, learner, conv_id) = setup().await;
        let service = svc(pool.clone());
        let a = seed_concept(&pool, "Rel-C").await;
        let cand = node("Rel-Candidate", 0.9);
        let cand_id = cand.id;
        service.propose_concept(learner, conv_id, &cand).await.unwrap();

        service
            .propose_relation(
                learner,
                conv_id,
                ConceptRef::Existing { concept_id: a },
                ConceptRef::Candidate { candidate_id: cand_id },
                "semantic",
                "related ideas",
            )
            .await
            .unwrap();
        assert_eq!(relation_count(&pool).await, 1);
    }

    #[tokio::test]
    async fn relation_gate_rejects_bad_type_self_unknown_and_empty_reason() {
        let (pool, learner, conv_id) = setup().await;
        let service = svc(pool.clone());
        let a = seed_concept(&pool, "Rel-D").await;
        let b = seed_concept(&pool, "Rel-E").await;
        let missing = Uuid::new_v4();
        let existing_a = || ConceptRef::Existing { concept_id: a };
        let existing_b = || ConceptRef::Existing { concept_id: b };

        for (from, to, rel, reason) in [
            (existing_a(), existing_b(), "causal", "ok"),
            (existing_a(), existing_a(), "dependency", "ok"),
            (existing_a(), ConceptRef::Existing { concept_id: missing }, "dependency", "ok"),
            (existing_a(), existing_b(), "dependency", "   "),
        ] {
            let err = service
                .propose_relation(learner, conv_id, from, to, rel, reason)
                .await
                .unwrap_err();
            assert!(err.to_string().contains("proposal rejected"), "got: {err}");
        }
        assert_eq!(relation_count(&pool).await, 0);
    }

    async fn audit_count(pool: &SqlitePool) -> i64 {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM graph_mutations")
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn admission_commits_node_verdict_and_audit_atomically() {
        let (pool, learner, conv_id) = setup().await;
        let service = svc(pool.clone());
        let n = node("Admit Me", 0.9);
        service.propose_concept(learner, conv_id, &n).await.unwrap();

        let concept_id = service.admit_concept_candidate(learner, n.id).await.unwrap();
        assert_eq!(concept_id, n.id);

        let stored =
            infrastructure::graph_repo::get_concept(&pool, concept_id).await.unwrap().unwrap();
        assert_eq!(stored.canonical_name, "Admit Me");

        let row: (String, Option<String>) =
            sqlx::query_as("SELECT status, evaluated_at FROM concept_candidates WHERE id = ?")
                .bind(n.id.to_string())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(row.0, "accepted");
        assert!(row.1.is_some());

        assert_eq!(audit_count(&pool).await, 1);
        let payload: String = sqlx::query_scalar("SELECT payload FROM graph_mutations LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(value["kind"], "concept_admission");
        assert_eq!(value["concept_id"], n.id.to_string());
    }

    #[tokio::test]
    async fn admission_rechecks_gate_and_writes_nothing_on_rejection() {
        let (pool, learner, conv_id) = setup().await;
        let service = svc(pool.clone());
        // Bypass the propose gate with a direct low-confidence insert.
        let weak_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO concept_candidates (id, learner_id, conversation_id, canonical_name, canonical_statement, world_confidence)
             VALUES (?, ?, ?, 'Shaky', 'Shaky statement.', 0.4)",
        )
        .bind(weak_id.to_string())
        .bind(learner.to_string())
        .bind(conv_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

        let err = service.admit_concept_candidate(learner, weak_id).await.unwrap_err();
        assert!(err.to_string().contains("admission gate"), "got: {err}");

        assert!(infrastructure::graph_repo::get_concept(&pool, weak_id).await.unwrap().is_none());
        let row: (String, Option<String>) =
            sqlx::query_as("SELECT status, rejection_reason FROM concept_candidates WHERE id = ?")
                .bind(weak_id.to_string())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(row.0, "rejected");
        assert!(row.1.is_some_and(|r| r.contains("0.4")));
        assert_eq!(audit_count(&pool).await, 0);
    }

    #[tokio::test]
    async fn double_admission_and_unknown_id_fail_cleanly() {
        let (pool, learner, conv_id) = setup().await;
        let service = svc(pool.clone());
        let n = node("Admit Once", 0.9);
        service.propose_concept(learner, conv_id, &n).await.unwrap();
        service.admit_concept_candidate(learner, n.id).await.unwrap();

        let err = service.admit_concept_candidate(learner, n.id).await.unwrap_err();
        assert!(err.to_string().contains("already decided"), "got: {err}");
        assert_eq!(audit_count(&pool).await, 1);

        let err = service.admit_concept_candidate(learner, Uuid::new_v4()).await.unwrap_err();
        assert!(err.to_string().contains("not found"), "got: {err}");
    }
}
