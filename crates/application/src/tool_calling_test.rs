#[cfg(test)]
mod tests {
    use crate::tutor_service::stream_tutor_turn;
    use domain::TutorEvent;
    use llm::FakeLlmClient;
    use sqlx::SqlitePool;

    #[tokio::test]
    async fn test_tool_calling_flow_with_fake_llm() {
        // This test simulates the tool calling flow.
        // Even with FakeLlmClient, we can ensure the harness is correct.
        // We will eventually need to test this with a real LLM/mock provider.
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();

        let _learner_id = crate::conversation_service::ensure_default_learner(&pool).await.unwrap();
        let conv =
            crate::conversation_service::create_conversation(&pool, Some("test")).await.unwrap();

        let llm = std::sync::Arc::new(FakeLlmClient::new("fake-tutor-1"));
        let mut rx =
            stream_tutor_turn(pool, conv.id, "I need help with a concept.".into(), llm, None)
                .await
                .unwrap();

        let mut events = Vec::new();
        while let Some(ev) = rx.recv().await {
            events.push(ev.unwrap());
        }

        // Ensure at least one text delta and completion
        assert!(events.iter().any(|e| matches!(e, TutorEvent::TextDelta { .. })));
        assert!(events.iter().any(|e| matches!(e, TutorEvent::TurnCompleted { .. })));
    }

    /// M4/B4: the inspect-before-teach loop is health-sensitive. The same
    /// scripted `get_weak_dependencies` call surfaces the weak prerequisite
    /// into the model's context only when learner confidence is low, proving
    /// prerequisite health (not just graph shape) drives the turn.
    async fn weak_dep_final_text(learner_confidence: f32) -> String {
        use domain::{ConceptNode, ConceptStatus};
        use infrastructure::graph_repo;

        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        let learner = crate::conversation_service::ensure_default_learner(&pool).await.unwrap();
        let conv =
            crate::conversation_service::create_conversation(&pool, Some("test")).await.unwrap();

        let prime = ConceptNode {
            id: uuid::Uuid::new_v4(),
            canonical_name: "Prime Number".into(),
            canonical_statement: "Prime Number statement.".into(),
            learner_statement: None,
            world_confidence: 1.0,
            status: ConceptStatus::Active,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let factor = ConceptNode {
            id: uuid::Uuid::new_v4(),
            canonical_name: "Factor".into(),
            canonical_statement: "Factor statement.".into(),
            learner_statement: None,
            world_confidence: 1.0,
            status: ConceptStatus::Active,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        for n in [&prime, &factor] {
            graph_repo::create_concept(&pool, n).await.unwrap();
        }
        sqlx::query(
            "INSERT INTO concept_relations (id, from_concept_id, to_concept_id, relation_type) VALUES (?, ?, ?, 'dependency')",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(prime.id.to_string())
        .bind(factor.id.to_string())
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO learner_concept_states (learner_id, concept_id, learner_confidence) VALUES (?, ?, ?)",
        )
        .bind(learner.to_string())
        .bind(factor.id.to_string())
        .bind(learner_confidence)
        .execute(&pool)
        .await
        .unwrap();

        let stub = std::sync::Arc::new(llm::StubLlmClient::with_tools(vec![(
            "get_weak_dependencies",
            serde_json::json!({ "concept_id": prime.id.to_string(), "threshold": 0.7 }),
        )]));
        let mut rx = stream_tutor_turn(pool, conv.id, "Teach me Prime Number.".into(), stub, None)
            .await
            .unwrap();

        let mut events = Vec::new();
        while let Some(ev) = rx.recv().await {
            events.push(ev.unwrap());
        }
        assert!(
            events.iter().any(|e| matches!(
                e,
                TutorEvent::ToolCallStarted { tool_name, .. } if tool_name == "get_weak_dependencies"
            )),
            "expected the inspect-before-teach tool call"
        );
        assert!(events.iter().any(|e| matches!(e, TutorEvent::TurnCompleted { .. })));
        events
            .iter()
            .filter_map(|e| match e {
                TutorEvent::TextDelta { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    #[tokio::test]
    async fn weak_prerequisite_reaches_model_context_only_when_weak() {
        let weak_text = weak_dep_final_text(0.3).await;
        assert!(weak_text.contains("Factor"), "weak prerequisite must surface: {weak_text}");
        let healthy_text = weak_dep_final_text(0.98).await;
        assert!(
            !healthy_text.contains("Factor"),
            "healthy prerequisite must stay out: {healthy_text}"
        );
    }

    /// M5/D4: teaching novel material yields pending candidates — never
    /// authoritative rows. Turn 1 proposes the concept; turn 2 links it to an
    /// existing concept via a candidate reference.
    #[tokio::test]
    async fn novel_material_yields_pending_candidates_never_authoritative() {
        use domain::{ConceptNode, ConceptStatus};

        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        let _learner = crate::conversation_service::ensure_default_learner(&pool).await.unwrap();
        let conv =
            crate::conversation_service::create_conversation(&pool, Some("test")).await.unwrap();

        let prime = ConceptNode {
            id: uuid::Uuid::new_v4(),
            canonical_name: "Prime Number".into(),
            canonical_statement: "Prime Number statement.".into(),
            learner_statement: None,
            world_confidence: 1.0,
            status: ConceptStatus::Active,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        infrastructure::graph_repo::create_concept(&pool, &prime).await.unwrap();

        async fn drain(
            rx: &mut tokio::sync::mpsc::Receiver<Result<TutorEvent, anyhow::Error>>,
        ) -> Vec<TutorEvent> {
            let mut events = Vec::new();
            while let Some(ev) = rx.recv().await {
                events.push(ev.unwrap());
            }
            events
        }

        // Turn 1: propose the novel concept.
        let stub = std::sync::Arc::new(llm::StubLlmClient::with_tools(vec![(
            "propose_concept",
            serde_json::json!({
                "canonical_name": "Quantum Tunneling",
                "canonical_statement": "A particle crossing an energy barrier it classically cannot.",
                "world_confidence": 0.9,
            }),
        )]));
        let mut rx = stream_tutor_turn(
            pool.clone(),
            conv.id,
            "Teach me Quantum Tunneling.".into(),
            stub,
            None,
        )
        .await
        .unwrap();
        let events = drain(&mut rx).await;
        assert!(events.iter().any(|e| matches!(
            e,
            TutorEvent::ToolCallStarted { tool_name, .. } if tool_name == "propose_concept"
        )));
        assert!(events.iter().any(|e| matches!(e, TutorEvent::TurnCompleted { .. })));

        let row: (String, f32, String) = sqlx::query_as(
            "SELECT canonical_name, world_confidence, status FROM concept_candidates WHERE canonical_name = 'Quantum Tunneling'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.0, "Quantum Tunneling");
        assert!((row.1 - 0.9).abs() < 1e-6);
        assert_eq!(row.2, "pending");
        let nodes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM concept_nodes")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(nodes, 1, "proposal must never write authoritative rows");

        // Turn 2: link the candidate to the existing concept.
        let candidate_id: String = sqlx::query_scalar("SELECT id FROM concept_candidates LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        let stub = std::sync::Arc::new(llm::StubLlmClient::with_tools(vec![(
            "propose_relation",
            serde_json::json!({
                "from": { "candidate_id": candidate_id },
                "to": { "concept_id": prime.id.to_string() },
                "relation_type": "dependency",
                "reason": "Tunneling builds on energy foundations",
            }),
        )]));
        let mut rx =
            stream_tutor_turn(pool.clone(), conv.id, "How does it connect?".into(), stub, None)
                .await
                .unwrap();
        let events = drain(&mut rx).await;
        assert!(events.iter().any(|e| matches!(
            e,
            TutorEvent::ToolCallStarted { tool_name, .. } if tool_name == "propose_relation"
        )));

        let row: (String, Option<String>, String) = sqlx::query_as(
            "SELECT status, to_concept_id, relation_type FROM relation_candidates LIMIT 1",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.0, "pending");
        assert_eq!(row.1, Some(prime.id.to_string()));
        assert_eq!(row.2, "dependency");
        let nodes: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM concept_nodes")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(nodes, 1, "relation proposals stay candidates too");
    }
}
