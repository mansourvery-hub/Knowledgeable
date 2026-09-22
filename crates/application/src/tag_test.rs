//! Phase 5 (bookmarks) tests: tag CRUD, validation, counts, cascade.

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;
    use uuid::Uuid;

    use crate::tag_service::{self, TagError};

    async fn setup() -> (SqlitePool, Uuid) {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        let learner = crate::conversation_service::ensure_default_learner(&pool).await.unwrap();
        (pool, learner)
    }

    async fn convo(pool: &SqlitePool) -> Uuid {
        crate::conversation_service::create_conversation(pool, Some("t")).await.unwrap().id
    }

    #[tokio::test]
    async fn creates_lists_and_counts() {
        let (pool, learner) = setup().await;
        let c1 = convo(&pool).await;
        let c2 = convo(&pool).await;

        let tag = tag_service::create_tag(&pool, learner, " math ", None).await.unwrap();
        assert_eq!(tag.tag, "math");
        assert_eq!(tag.count, 0);
        assert_eq!(tag.position, 0);

        tag_service::set_conversation_tags(&pool, learner, c1, &["math".into()]).await.unwrap();
        tag_service::set_conversation_tags(&pool, learner, c2, &["math".into(), "new".into()])
            .await
            .unwrap();

        let tags = tag_service::list_tags(&pool, learner).await.unwrap();
        assert_eq!(tags.len(), 2);
        // Position order: math (0) before auto-created new (1).
        assert_eq!(tags[0].tag, "math");
        assert_eq!(tags[0].count, 2);
        assert_eq!(tags[1].tag, "new");
        assert_eq!(tags[1].count, 1);
    }

    #[tokio::test]
    async fn rejects_bad_names_and_duplicates() {
        let (pool, learner) = setup().await;
        for bad in ["", "   "] {
            assert!(matches!(
                tag_service::create_tag(&pool, learner, bad, None).await,
                Err(TagError::EmptyName)
            ));
        }
        assert!(matches!(
            tag_service::create_tag(&pool, learner, &"x".repeat(101), None).await,
            Err(TagError::NameTooLong)
        ));
        tag_service::create_tag(&pool, learner, "dup", None).await.unwrap();
        assert!(matches!(
            tag_service::create_tag(&pool, learner, "dup", None).await,
            Err(TagError::AlreadyExists(_))
        ));
    }

    #[tokio::test]
    async fn rename_moves_membership_atomically() {
        let (pool, learner) = setup().await;
        let c = convo(&pool).await;
        tag_service::create_tag(&pool, learner, "old", None).await.unwrap();
        tag_service::set_conversation_tags(&pool, learner, c, &["old".into()]).await.unwrap();

        let renamed =
            tag_service::rename_tag(&pool, learner, "old", Some("new"), None, None).await.unwrap();
        assert_eq!(renamed.tag, "new");
        assert_eq!(
            tag_service::tags_for_conversation(&pool, learner, c).await.unwrap(),
            vec!["new".to_string()]
        );
        assert!(tag_service::list_tags(&pool, learner)
            .await
            .unwrap()
            .iter()
            .all(|t| t.tag != "old"));

        // Rename onto an existing name conflicts; missing tag 404s.
        tag_service::create_tag(&pool, learner, "other", None).await.unwrap();
        assert!(matches!(
            tag_service::rename_tag(&pool, learner, "new", Some("other"), None, None).await,
            Err(TagError::AlreadyExists(_))
        ));
        assert!(matches!(
            tag_service::rename_tag(&pool, learner, "ghost", Some("x"), None, None).await,
            Err(TagError::NotFound(_))
        ));
    }

    #[tokio::test]
    async fn delete_drops_membership_and_conversation_delete_cascades() {
        let (pool, learner) = setup().await;
        let c = convo(&pool).await;
        tag_service::create_tag(&pool, learner, "gone", None).await.unwrap();
        tag_service::set_conversation_tags(&pool, learner, c, &["gone".into()]).await.unwrap();

        tag_service::delete_tag(&pool, learner, "gone").await.unwrap();
        assert!(tag_service::tags_for_conversation(&pool, learner, c).await.unwrap().is_empty());
        assert!(matches!(
            tag_service::delete_tag(&pool, learner, "gone").await,
            Err(TagError::NotFound(_))
        ));

        tag_service::create_tag(&pool, learner, "kept", None).await.unwrap();
        tag_service::set_conversation_tags(&pool, learner, c, &["kept".into()]).await.unwrap();
        crate::conversation_service::delete_conversation(&pool, c).await.unwrap();
        let tags = tag_service::list_tags(&pool, learner).await.unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].tag, "kept");
        assert_eq!(tags[0].count, 0);
    }

    #[tokio::test]
    async fn set_replaces_and_rejects_unknown_conversation() {
        let (pool, learner) = setup().await;
        let c = convo(&pool).await;
        tag_service::set_conversation_tags(&pool, learner, c, &["a".into(), "b".into()])
            .await
            .unwrap();
        let tags = tag_service::set_conversation_tags(&pool, learner, c, &["b".into(), "c".into()])
            .await
            .unwrap();
        assert_eq!(tags, vec!["b".to_string(), "c".to_string()]);
        assert!(matches!(
            tag_service::set_conversation_tags(&pool, learner, Uuid::new_v4(), &["x".into()]).await,
            Err(TagError::ConversationNotFound)
        ));
    }
}
