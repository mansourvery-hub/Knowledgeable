#[cfg(test)]
mod tests {
    use crate::tutor_service::stream_tutor_turn;
    use chrono::Utc;
    use domain::TutorEvent;
    use llm::FakeLlmClient;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_stream_tutor_turn_flow() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();

        let learner_id = crate::conversation_service::ensure_default_learner(&pool).await.unwrap();
        let conv =
            crate::conversation_service::create_conversation(&pool, Some("test")).await.unwrap();

        let llm = std::sync::Arc::new(FakeLlmClient::new("fake-tutor-1"));
        let mut rx =
            stream_tutor_turn(pool, conv.id, "Hello tutor!".into(), llm, None).await.unwrap();

        let mut events = Vec::new();
        while let Some(ev) = rx.recv().await {
            events.push(ev.unwrap());
        }

        assert!(events.iter().any(|e| matches!(e, TutorEvent::TextDelta { .. })));
        assert!(events.iter().any(|e| matches!(e, TutorEvent::TurnCompleted { .. })));
    }
}

#[cfg(test)]
mod argument_parsing_tests {
    use crate::tutor_service::parse_tool_arguments;

    #[test]
    fn whole_payload_preferred() {
        let v = parse_tool_arguments(r#"{"concept_id":"abc"}"#);
        assert_eq!(v["concept_id"], "abc");
    }

    #[test]
    fn duplicated_resends_recover_first_copy() {
        // Gemini re-sends complete arguments across chunks; concatenation
        // must not corrupt the call or the echoed history (which Gemini
        // validates as JSON, 400 otherwise).
        let one = r#"{"concept_id":"1343224e-9399-4077-9949-99be10e076b4"}"#;
        let v = parse_tool_arguments(&format!("{one}{one}"));
        assert_eq!(v["concept_id"], "1343224e-9399-4077-9949-99be10e076b4");
    }

    #[test]
    fn total_garbage_stays_string_for_downstream_validation() {
        let v = parse_tool_arguments("not json at all");
        assert_eq!(v, serde_json::json!("not json at all"));
    }
}
