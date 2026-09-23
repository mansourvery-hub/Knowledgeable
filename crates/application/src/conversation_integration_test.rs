#[cfg(test)]
mod tests {
    use crate::tutor_service::stream_tutor_turn;
    use llm::FakeLlmClient;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_multi_turn_conversation_flow() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        let _learner_id = crate::conversation_service::ensure_default_learner(&pool).await.unwrap();
        let conv =
            crate::conversation_service::create_conversation(&pool, Some("math")).await.unwrap();

        let llm = Arc::new(FakeLlmClient::new("tutor"));

        // Turn 1: User asks about prime numbers
        let mut rx1 = stream_tutor_turn(
            pool.clone(),
            conv.id,
            "What is a prime number?".into(),
            llm.clone(),
            None,
        )
        .await
        .unwrap();
        while rx1.recv().await.is_some() {}

        // Turn 2: User asks follow up
        let mut rx2 = stream_tutor_turn(
            pool.clone(),
            conv.id,
            "Is 4 a prime number?".into(),
            llm.clone(),
            None,
        )
        .await
        .unwrap();
        while rx2.recv().await.is_some() {}

        // Verify history count (2 user + 2 assistant messages = 4)
        let msgs = crate::conversation_service::list_messages(&pool, conv.id).await.unwrap();
        assert_eq!(msgs.len(), 4);
    }
}
