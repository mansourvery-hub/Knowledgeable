//! M6 observation tests: validation gate (E1), apply transaction (E2).

#[cfg(test)]
mod tests {
    use crate::tutor_service::stream_tutor_turn;
    use domain::{ConceptNode, ConceptStatus, TutorEvent};
    use sqlx::SqlitePool;
    use uuid::Uuid;

    async fn setup() -> (SqlitePool, Uuid) {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        let _learner = crate::conversation_service::ensure_default_learner(&pool).await.unwrap();
        let conv =
            crate::conversation_service::create_conversation(&pool, Some("test")).await.unwrap();
        (pool, conv.id)
    }

    async fn seed_concept(pool: &SqlitePool, name: &str) -> Uuid {
        let n = ConceptNode {
            id: Uuid::new_v4(),
            canonical_name: name.into(),
            canonical_statement: format!("{name} statement."),
            learner_statement: None,
            world_confidence: 1.0,
            status: ConceptStatus::Active,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        infrastructure::graph_repo::create_concept(pool, &n).await.unwrap();
        n.id
    }

    async fn run_observation_turn(
        pool: SqlitePool,
        conv_id: Uuid,
        args: serde_json::Value,
    ) -> (Vec<TutorEvent>, String) {
        let stub =
            std::sync::Arc::new(llm::StubLlmClient::with_tools(vec![("log_observation", args)]));
        let mut rx =
            stream_tutor_turn(pool, conv_id, "Turn with an observation.".into(), stub, None)
                .await
                .unwrap();
        let mut events = Vec::new();
        while let Some(ev) = rx.recv().await {
            events.push(ev.unwrap());
        }
        let text = events
            .iter()
            .filter_map(|e| match e {
                TutorEvent::TextDelta { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");
        (events, text)
    }

    async fn observation_count(pool: &SqlitePool) -> i64 {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM learner_observations")
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn valid_observation_logged() {
        let (pool, conv_id) = setup().await;
        let concept = seed_concept(&pool, "Observed Concept").await;

        let (events, _text) = run_observation_turn(
            pool.clone(),
            conv_id,
            serde_json::json!({
                "concept_id": concept.to_string(),
                "observation_type": "confusion",
                "confidence_delta": -0.2,
                "evidence": "Learner mixed up the definition.",
            }),
        )
        .await;

        assert!(events.iter().any(|e| matches!(
            e,
            TutorEvent::ToolCallStarted { tool_name, .. } if tool_name == "log_observation"
        )));
        assert!(events.iter().any(|e| matches!(e, TutorEvent::TurnCompleted { .. })));
        assert_eq!(observation_count(&pool).await, 1);
    }

    #[tokio::test]
    async fn invalid_observations_rejected_without_rows() {
        let (pool, conv_id) = setup().await;
        let concept = seed_concept(&pool, "Another Concept").await;
        let missing = Uuid::new_v4();

        for (label, args) in [
            (
                "unknown concept",
                serde_json::json!({
                    "concept_id": missing.to_string(),
                    "observation_type": "confusion",
                    "evidence": "Something seemed off.",
                }),
            ),
            (
                "delta out of range",
                serde_json::json!({
                    "concept_id": concept.to_string(),
                    "observation_type": "confusion",
                    "confidence_delta": 5.0,
                    "evidence": "Way off.",
                }),
            ),
            (
                "empty evidence",
                serde_json::json!({
                    "concept_id": concept.to_string(),
                    "observation_type": "confusion",
                    "evidence": "   ",
                }),
            ),
            (
                "bad type",
                serde_json::json!({
                    "concept_id": concept.to_string(),
                    "observation_type": "vibes",
                    "evidence": "Unclear.",
                }),
            ),
        ] {
            let (_events, text) = run_observation_turn(pool.clone(), conv_id, args).await;
            assert!(
                text.contains("invalid") || text.contains("required"),
                "{label} must surface a validation error, got: {text}"
            );
        }
        assert_eq!(observation_count(&pool).await, 0);
    }

    fn observation(
        learner: Uuid,
        conv_id: Uuid,
        concept: Option<Uuid>,
        kind: domain::ObservationType,
        delta: Option<f32>,
    ) -> domain::LearnerObservation {
        domain::LearnerObservation {
            id: Uuid::new_v4(),
            learner_id: learner,
            conversation_id: conv_id,
            concept_id: concept,
            observation_type: kind,
            confidence_delta: delta,
            evidence: "test evidence".to_string(),
            created_at: chrono::Utc::now(),
        }
    }

    async fn confidence_of(pool: &SqlitePool, learner: Uuid, concept: Uuid) -> Option<f32> {
        sqlx::query_scalar::<_, f32>(
            "SELECT learner_confidence FROM learner_concept_states WHERE learner_id = ? AND concept_id = ?",
        )
        .bind(learner.to_string())
        .bind(concept.to_string())
        .fetch_optional(pool)
        .await
        .unwrap()
    }

    async fn open_reviews(pool: &SqlitePool, learner: Uuid, concept: Uuid) -> i64 {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM review_items WHERE learner_id = ? AND concept_id = ? AND status IN ('eligible','in_progress')",
        )
        .bind(learner.to_string())
        .bind(concept.to_string())
        .fetch_optional(pool)
        .await
        .unwrap()
        .unwrap_or(0)
    }

    #[tokio::test]
    async fn confusion_drops_confidence_and_opens_review() {
        let (pool, conv_id) = setup().await;
        let learner = crate::conversation_service::ensure_default_learner(&pool).await.unwrap();
        let concept = seed_concept(&pool, "Shaky Concept").await;
        let svc = crate::graph_service::GraphService::new(std::sync::Arc::new(pool.clone()));

        let out = svc
            .apply_observation(&observation(
                learner,
                conv_id,
                Some(concept),
                domain::ObservationType::Confusion,
                Some(-0.2),
            ))
            .await
            .unwrap();

        // Neutral prior 0.5 − 0.2.
        assert!((out.learner_confidence.unwrap() - 0.3).abs() < 1e-6);
        assert_eq!(out.review_eligible, Some(true));
        assert!((confidence_of(&pool, learner, concept).await.unwrap() - 0.3).abs() < 1e-6);
        assert_eq!(open_reviews(&pool, learner, concept).await, 1);
        assert_eq!(observation_count(&pool).await, 1);
    }

    #[tokio::test]
    async fn repair_resolves_review_and_clamps_bounds() {
        let (pool, conv_id) = setup().await;
        let learner = crate::conversation_service::ensure_default_learner(&pool).await.unwrap();
        let concept = seed_concept(&pool, "Repair Concept").await;
        let svc = crate::graph_service::GraphService::new(std::sync::Arc::new(pool.clone()));

        svc.apply_observation(&observation(
            learner,
            conv_id,
            Some(concept),
            domain::ObservationType::Confusion,
            Some(-0.2),
        ))
        .await
        .unwrap();
        assert_eq!(open_reviews(&pool, learner, concept).await, 1);

        // 0.3 + 0.7 clamps to 1.0 and resolves the review.
        let out = svc
            .apply_observation(&observation(
                learner,
                conv_id,
                Some(concept),
                domain::ObservationType::Understands,
                Some(0.7),
            ))
            .await
            .unwrap();
        assert_eq!(out.learner_confidence, Some(1.0));
        assert_eq!(out.review_eligible, Some(false));
        assert_eq!(open_reviews(&pool, learner, concept).await, 0);

        // Floor clamp: 1.0 − 1.0.
        let out = svc
            .apply_observation(&observation(
                learner,
                conv_id,
                Some(concept),
                domain::ObservationType::Misconception,
                Some(-1.0),
            ))
            .await
            .unwrap();
        assert_eq!(out.learner_confidence, Some(0.0));
    }

    #[tokio::test]
    async fn observation_without_delta_records_only() {
        let (pool, conv_id) = setup().await;
        let learner = crate::conversation_service::ensure_default_learner(&pool).await.unwrap();
        let concept = seed_concept(&pool, "Noted Concept").await;
        let svc = crate::graph_service::GraphService::new(std::sync::Arc::new(pool.clone()));

        let out = svc
            .apply_observation(&observation(
                learner,
                conv_id,
                Some(concept),
                domain::ObservationType::NewUnderstanding,
                None,
            ))
            .await
            .unwrap();
        assert_eq!(out.learner_confidence, None);
        assert_eq!(observation_count(&pool).await, 1);
        assert!(confidence_of(&pool, learner, concept).await.is_none());
    }

    #[tokio::test]
    async fn tool_result_reports_new_confidence() {
        let (pool, conv_id) = setup().await;
        let concept = seed_concept(&pool, "Reported Concept").await;

        let (_events, text) = run_observation_turn(
            pool,
            conv_id,
            serde_json::json!({
                "concept_id": concept.to_string(),
                "observation_type": "confusion",
                "confidence_delta": -0.2,
                "evidence": "Mixed up the terms.",
            }),
        )
        .await;
        assert!(text.contains("learner_confidence"), "got: {text}");
        assert!(text.contains("review_eligible"), "got: {text}");
    }
}
