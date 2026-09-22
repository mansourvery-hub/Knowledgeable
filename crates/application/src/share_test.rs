//! Phase 5 (minimal share, F27) tests: links, scope, fork, revoke.

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;
    use uuid::Uuid;

    use crate::share_service;

    async fn setup() -> (SqlitePool, Uuid) {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        let learner = crate::conversation_service::ensure_default_learner(&pool).await.unwrap();
        (pool, learner)
    }

    async fn convo(pool: &SqlitePool, title: &str, texts: &[&str]) -> Uuid {
        let conv =
            crate::conversation_service::create_conversation(pool, Some(title)).await.unwrap();
        for (i, text) in texts.iter().enumerate() {
            let role =
                if i % 2 == 0 { domain::MessageRole::User } else { domain::MessageRole::Assistant };
            crate::conversation_service::add_message(pool, conv.id, role, text).await.unwrap();
        }
        conv.id
    }

    #[tokio::test]
    async fn create_read_scope_and_revoke() {
        let (pool, _) = setup().await;
        let id = convo(&pool, "Share Me", &["q one", "a one", "q two", "a two"]).await;

        let link = share_service::create_link(&pool, id, None).await.unwrap().unwrap();
        assert_ne!(link.share_id.to_string(), id.to_string());

        let found = share_service::link_for_conversation(&pool, id).await.unwrap().unwrap();
        assert_eq!(found.share_id, link.share_id);

        // Full payload without scope.
        let messages = share_service::link_messages(&pool, &link).await.unwrap();
        assert_eq!(messages.len(), 4);

        // Scoped to the second message: prefix of two.
        let second_id = messages[1].id;
        let scoped = share_service::retarget_link(&pool, link.share_id, Some(second_id))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(scoped.target_message_id, Some(second_id));
        let scoped_messages = share_service::link_messages(&pool, &scoped).await.unwrap();
        assert_eq!(scoped_messages.len(), 2);

        assert!(share_service::delete_link(&pool, link.share_id).await.unwrap());
        assert!(share_service::get_link(&pool, link.share_id).await.unwrap().is_none());

        // Unknown conversation yields no link.
        assert!(share_service::create_link(&pool, Uuid::new_v4(), None).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn fork_copies_scoped_prefix() {
        let (pool, _) = setup().await;
        let id = convo(&pool, "Fork Me", &["q one", "a one", "q two", "a two"]).await;
        let link = share_service::create_link(&pool, id, None).await.unwrap().unwrap();

        // Index 1 → first two messages.
        let (copy, messages) =
            share_service::fork_link(&pool, &link, Some(1)).await.unwrap().unwrap();
        assert_ne!(copy.id, id);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content, "q one");

        // No index → everything.
        let (_, all) = share_service::fork_link(&pool, &link, None).await.unwrap().unwrap();
        assert_eq!(all.len(), 4);

        // Gone source → None (revoke the link's conversation by deleting it).
        crate::conversation_service::delete_conversation(&pool, id).await.unwrap();
        assert!(share_service::get_link(&pool, link.share_id).await.unwrap().is_none());
    }
}
