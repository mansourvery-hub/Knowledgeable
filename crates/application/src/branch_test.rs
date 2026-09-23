//! Branching tests for Phase 5 message tree

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;
    use uuid::Uuid;

    async fn setup() -> (SqlitePool, Uuid) {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        let learner = crate::conversation_service::ensure_default_learner(&pool).await.unwrap();
        (pool, learner)
    }

    async fn convo(pool: &SqlitePool) -> Uuid {
        crate::conversation_service::create_conversation(pool, Some("branch test"))
            .await
            .unwrap()
            .id
    }

    #[tokio::test]
    async fn backfill_equals_old_synthesis() {
        let (pool, _) = setup().await;
        let id = convo(&pool).await;
        // Create linear history via add_message (tail-append)
        let m1 =
            crate::conversation_service::add_message(&pool, id, domain::MessageRole::User, "u1")
                .await
                .unwrap();
        let m2 = crate::conversation_service::add_message(
            &pool,
            id,
            domain::MessageRole::Assistant,
            "a1",
        )
        .await
        .unwrap();
        let m3 =
            crate::conversation_service::add_message(&pool, id, domain::MessageRole::User, "u2")
                .await
                .unwrap();
        assert_eq!(m1.parent_message_id, None);
        assert_eq!(m2.parent_message_id, Some(m1.id));
        assert_eq!(m3.parent_message_id, Some(m2.id));
        // List returns same chain
        let msgs = crate::conversation_service::list_messages(&pool, id).await.unwrap();
        assert_eq!(msgs[0].parent_message_id, None);
        assert_eq!(msgs[1].parent_message_id, Some(msgs[0].id));
        assert_eq!(msgs[2].parent_message_id, Some(msgs[1].id));
    }

    #[tokio::test]
    async fn sibling_branch_via_explicit_parent() {
        let (pool, _) = setup().await;
        let id = convo(&pool).await;
        let u1 = infrastructure::conversation_repo::create_message_with_parent(
            &pool,
            id,
            domain::MessageRole::User,
            "u1",
            None,
        )
        .await
        .unwrap();
        let a1 = infrastructure::conversation_repo::create_message_with_parent(
            &pool,
            id,
            domain::MessageRole::Assistant,
            "a1",
            Some(u1.id),
        )
        .await
        .unwrap();
        // Branch: second user also parents to None (sibling of u1), second assistant parents to u2
        let u2 = infrastructure::conversation_repo::create_message_with_parent(
            &pool,
            id,
            domain::MessageRole::User,
            "u2",
            None,
        )
        .await
        .unwrap();
        let a2 = infrastructure::conversation_repo::create_message_with_parent(
            &pool,
            id,
            domain::MessageRole::Assistant,
            "a2",
            Some(u2.id),
        )
        .await
        .unwrap();
        assert_eq!(u2.parent_message_id, None);
        assert_eq!(a2.parent_message_id, Some(u2.id));
        // Both assistants share same grandparent? No, distinct users.
        assert_ne!(u1.id, u2.id);
        assert_ne!(a1.id, a2.id);
        let msgs = crate::conversation_service::list_messages(&pool, id).await.unwrap();
        assert_eq!(msgs.len(), 4);
        // Verify siblings exist
        let parents: Vec<Option<Uuid>> = msgs.iter().map(|m| m.parent_message_id).collect();
        assert!(parents.contains(&None));
        assert!(parents.contains(&Some(u1.id)));
        assert!(parents.contains(&Some(u2.id)));
    }

    #[tokio::test]
    async fn unknown_parent_degrades_to_tail() {
        let (pool, _) = setup().await;
        let id = convo(&pool).await;
        let m1 =
            crate::conversation_service::add_message(&pool, id, domain::MessageRole::User, "u1")
                .await
                .unwrap();
        // Use tutor turn with unknown parent
        let pool_clone = pool.clone();
        let llm = std::sync::Arc::new(llm::FakeLlmClient::new("test"));
        let fake_parent = Uuid::new_v4();
        let (user_msg, _, mut rx) = crate::tutor_service::begin_tutor_turn_with_parent(
            pool_clone,
            id,
            "hello via unknown parent".into(),
            Some(fake_parent),
            llm,
            None,
        )
        .await
        .unwrap();
        // Should fallback to tail (m1), not the fake
        assert_eq!(user_msg.parent_message_id, Some(m1.id));
        // Drain to persist assistant
        while let Some(ev) = rx.recv().await {
            if matches!(
                ev,
                Ok(domain::TutorEvent::TurnCompleted { .. }) | Ok(domain::TutorEvent::Error { .. })
            ) {
                break;
            }
        }
        // Ensure no panic and messages exist
        let msgs = crate::conversation_service::list_messages(&pool, id).await.unwrap();
        assert!(msgs.len() >= 3);
    }
}
