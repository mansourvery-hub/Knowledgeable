//! M6/E3 tests: decay maintenance pass.

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use domain::{ConceptNode, ConceptStatus};
    use sqlx::SqlitePool;
    use uuid::Uuid;

    async fn setup() -> (SqlitePool, Uuid) {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
        let learner = crate::conversation_service::ensure_default_learner(&pool).await.unwrap();
        (pool, learner)
    }

    async fn seed_state(
        pool: &SqlitePool,
        learner: Uuid,
        name: &str,
        confidence: f32,
        reinforced_days_ago: Option<i64>,
    ) -> Uuid {
        let node = ConceptNode {
            id: Uuid::new_v4(),
            canonical_name: name.into(),
            canonical_statement: format!("{name} statement."),
            learner_statement: None,
            world_confidence: 1.0,
            status: ConceptStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        infrastructure::graph_repo::create_concept(pool, &node).await.unwrap();
        infrastructure::learner_repo::update_learner_confidence(pool, learner, node.id, confidence)
            .await
            .unwrap();
        if let Some(days) = reinforced_days_ago {
            // Backdate the evidence clock wholesale (anchor AND last update)
            // so the incremental scheme sees the full elapsed span.
            let anchor = (Utc::now() - Duration::days(days)).to_rfc3339();
            sqlx::query(
                "UPDATE learner_concept_states SET last_reinforced_at = ?, last_evaluated_at = ?, updated_at = ?, next_decay_at = NULL WHERE learner_id = ? AND concept_id = ?",
            )
            .bind(&anchor)
            .bind(&anchor)
            .bind(&anchor)
            .bind(learner.to_string())
            .bind(node.id.to_string())
            .execute(pool)
            .await
            .unwrap();
        }
        node.id
    }

    async fn confidence_of(pool: &SqlitePool, learner: Uuid, concept: Uuid) -> f32 {
        sqlx::query_scalar::<_, f32>(
            "SELECT learner_confidence FROM learner_concept_states WHERE learner_id = ? AND concept_id = ?",
        )
        .bind(learner.to_string())
        .bind(concept.to_string())
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn grace_period_values_untouched_but_hinted() {
        let (pool, learner) = setup().await;
        // Reinforced 1 year ago: inside the 2-year grace period.
        let concept = seed_state(&pool, learner, "Fresh Concept", 0.9, Some(365)).await;

        let decayed = infrastructure::learner_repo::apply_decay(
            &pool,
            learner,
            Utc::now(),
            domain::DEFAULT_DECAY_GRACE_YEARS,
            domain::DEFAULT_DECAY_HALF_LIFE_YEARS,
        )
        .await
        .unwrap();

        assert_eq!(decayed, 0);
        assert!((confidence_of(&pool, learner, concept).await - 0.9).abs() < 1e-6);
        let hint: Option<String> = sqlx::query_scalar(
            "SELECT next_decay_at FROM learner_concept_states WHERE learner_id = ? AND concept_id = ?",
        )
        .bind(learner.to_string())
        .bind(concept.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(hint.is_some(), "in-grace rows get a recheck hint");
    }

    #[tokio::test]
    async fn old_values_decay_and_converge() {
        let (pool, learner) = setup().await;
        // Reinforced 6 years ago: 4 years past grace, half-life 12.
        let concept = seed_state(&pool, learner, "Old Concept", 1.0, Some(6 * 365)).await;

        let decayed = infrastructure::learner_repo::apply_decay(
            &pool,
            learner,
            Utc::now(),
            domain::DEFAULT_DECAY_GRACE_YEARS,
            domain::DEFAULT_DECAY_HALF_LIFE_YEARS,
        )
        .await
        .unwrap();

        assert_eq!(decayed, 1);
        let expected = 0.5_f32.powf(4.0 / 12.0);
        let actual = confidence_of(&pool, learner, concept).await;
        assert!((actual - expected).abs() < 0.02, "got {actual}, want ~{expected}");

        // Anchor preserved: a second pass recomputes the same value (no write).
        let decayed_again = infrastructure::learner_repo::apply_decay(
            &pool,
            learner,
            Utc::now(),
            domain::DEFAULT_DECAY_GRACE_YEARS,
            domain::DEFAULT_DECAY_HALF_LIFE_YEARS,
        )
        .await
        .unwrap();
        assert_eq!(decayed_again, 0);
        assert!((confidence_of(&pool, learner, concept).await - expected).abs() < 0.02);
    }
}
