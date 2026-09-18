use chrono::Utc;
use domain::LearnerObservation;
use sqlx::SqlitePool;

/// Outcome of applying one observation: the learner's resulting confidence
/// (`None` when the observation names no concept or carries no delta) and
/// whether the concept is review-eligible afterwards.
#[derive(Debug, Clone, PartialEq)]
pub struct AppliedObservation {
    pub learner_confidence: Option<f32>,
    pub review_eligible: Option<bool>,
}

/// Neutral starting confidence for a first-seen concept (data_models §19:
/// confidence changes come from explicit evidence; the neutral prior keeps a
/// single observation from pinning a concept to an extreme).
pub const NEUTRAL_CONFIDENCE: f32 = 0.5;

pub async fn log_observation(
    pool: &SqlitePool,
    observation: &LearnerObservation,
) -> Result<(), sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    let id_s = observation.id.to_string();
    let learner_s = observation.learner_id.to_string();
    let conv_s = observation.conversation_id.to_string();
    let concept_s = observation.concept_id.map(|id| id.to_string());

    sqlx::query(
        "INSERT INTO learner_observations (id, learner_id, conversation_id, concept_id, observation_type, confidence_delta, evidence, created_at) 
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id_s)
    .bind(learner_s)
    .bind(conv_s)
    .bind(concept_s)
    .bind(observation.observation_type.as_str())
    .bind(observation.confidence_delta)
    .bind(&observation.evidence)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}

/// Log an observation and apply its confidence evidence atomically (M6/E2).
///
/// One transaction: the observation row, the `learner_concept_states` upsert
/// (created at the neutral prior when first seen, clamped to [0,1], evidence
/// timestamp maintained), and the `review_items` sync (open an `eligible`
/// row below `HEALTHY_THRESHOLD`, resolve open rows once healthy again).
/// Observations without a concept or without a delta only record evidence.
pub async fn apply_observation(
    pool: &SqlitePool,
    observation: &LearnerObservation,
) -> Result<AppliedObservation, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO learner_observations (id, learner_id, conversation_id, concept_id, observation_type, confidence_delta, evidence, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(observation.id.to_string())
    .bind(observation.learner_id.to_string())
    .bind(observation.conversation_id.to_string())
    .bind(observation.concept_id.map(|id| id.to_string()))
    .bind(observation.observation_type.as_str())
    .bind(observation.confidence_delta)
    .bind(&observation.evidence)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    let (Some(concept_id), Some(delta)) = (observation.concept_id, observation.confidence_delta)
    else {
        tx.commit().await?;
        return Ok(AppliedObservation { learner_confidence: None, review_eligible: None });
    };

    let current: Option<f32> = sqlx::query_scalar(
        "SELECT learner_confidence FROM learner_concept_states WHERE learner_id = ? AND concept_id = ?",
    )
    .bind(observation.learner_id.to_string())
    .bind(concept_id.to_string())
    .fetch_optional(&mut *tx)
    .await?;
    let updated = (current.unwrap_or(NEUTRAL_CONFIDENCE) + delta).clamp(0.0, 1.0);

    if current.is_some() {
        if delta > 0.0 {
            sqlx::query(
                "UPDATE learner_concept_states SET learner_confidence = ?, last_evaluated_at = ?, last_reinforced_at = ?, updated_at = ? WHERE learner_id = ? AND concept_id = ?",
            )
            .bind(updated)
            .bind(&now)
            .bind(&now)
            .bind(&now)
            .bind(observation.learner_id.to_string())
            .bind(concept_id.to_string())
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query(
                "UPDATE learner_concept_states SET learner_confidence = ?, last_evaluated_at = ?, updated_at = ? WHERE learner_id = ? AND concept_id = ?",
            )
            .bind(updated)
            .bind(&now)
            .bind(&now)
            .bind(observation.learner_id.to_string())
            .bind(concept_id.to_string())
            .execute(&mut *tx)
            .await?;
        }
    } else {
        sqlx::query(
            "INSERT INTO learner_concept_states (learner_id, concept_id, learner_confidence, last_evaluated_at, last_reinforced_at, updated_at, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(observation.learner_id.to_string())
        .bind(concept_id.to_string())
        .bind(updated)
        .bind(&now)
        .bind(if delta > 0.0 { Some(now.clone()) } else { None })
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }

    if updated < domain::HEALTHY_THRESHOLD {
        let open: Option<(i64,)> = sqlx::query_as(
            "SELECT 1 FROM review_items WHERE learner_id = ? AND concept_id = ? AND status IN ('eligible','in_progress') LIMIT 1",
        )
        .bind(observation.learner_id.to_string())
        .bind(concept_id.to_string())
        .fetch_optional(&mut *tx)
        .await?;
        if open.is_none() {
            sqlx::query(
                "INSERT INTO review_items (id, learner_id, concept_id, status, reason) VALUES (?, ?, ?, 'eligible', ?)",
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(observation.learner_id.to_string())
            .bind(concept_id.to_string())
            .bind(format!("learner_confidence {updated:.2} below {}", domain::HEALTHY_THRESHOLD))
            .execute(&mut *tx)
            .await?;
        }
    } else {
        sqlx::query(
            "UPDATE review_items SET status = 'resolved', resolved_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')
             WHERE learner_id = ? AND concept_id = ? AND status IN ('eligible','in_progress')",
        )
        .bind(observation.learner_id.to_string())
        .bind(concept_id.to_string())
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(AppliedObservation {
        learner_confidence: Some(updated),
        review_eligible: Some(updated < domain::HEALTHY_THRESHOLD),
    })
}
