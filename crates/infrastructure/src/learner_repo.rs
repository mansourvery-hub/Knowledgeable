use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

fn parse_dt(s: &str) -> DateTime<Utc> {
    // Unparsable timestamps anchor at now: the row is treated as fresh and
    // skipped rather than decayed on corrupt data (safe direction).
    s.parse::<DateTime<Utc>>().unwrap_or_else(|_| Utc::now())
}

/// One learner state row as needed by the decay pass.
type StateRow = (String, f32, Option<String>, String, String, Option<String>);

pub async fn update_learner_confidence(
    pool: &SqlitePool,
    learner_id: Uuid,
    concept_id: Uuid,
    new_confidence: f32,
) -> Result<(), sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO learner_concept_states (learner_id, concept_id, learner_confidence, updated_at, created_at) 
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(learner_id, concept_id) DO UPDATE SET 
         learner_confidence = excluded.learner_confidence,
         updated_at = excluded.updated_at"
    )
    .bind(learner_id.to_string())
    .bind(concept_id.to_string())
    .bind(new_confidence)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}

/// Current learner confidence for one concept, if any state exists.
pub async fn get_confidence(
    pool: &SqlitePool,
    learner_id: Uuid,
    concept_id: Uuid,
) -> Result<Option<f32>, sqlx::Error> {
    sqlx::query_scalar::<_, f32>(
        "SELECT learner_confidence FROM learner_concept_states WHERE learner_id = ? AND concept_id = ?",
    )
    .bind(learner_id.to_string())
    .bind(concept_id.to_string())
    .fetch_optional(pool)
    .await
}

/// Long-term decay maintenance pass (M6/E3).
///
/// Exponential decay composes multiplicatively, so each pass decays
/// incrementally from the row's last update instead of re-decaying the
/// stored value from the evidence anchor (which would compound). Concretely,
/// past the grace period (`anchor + grace_years`, where the anchor is the
/// evidence timestamp):
/// `new = current * 0.5^((now − max(updated_at, grace_end)) / half_life)`.
/// Evidence writes move `updated_at` (see `apply_observation`), so fresh
/// evidence correctly restarts the incremental clock while the grace gate
/// still honors the original anchor.
///
/// `next_decay_at` is purely a scheduler hint: anchor plus grace for in-grace
/// rows, one day out after a decay write; rows hinted in the future are
/// skipped, so rapid re-passes converge. Timestamps are never rewritten to
/// fake freshness. Returns the number of decayed rows.
pub async fn apply_decay(
    pool: &SqlitePool,
    learner_id: Uuid,
    now: DateTime<Utc>,
    grace_years: f32,
    half_life_years: f32,
) -> Result<usize, sqlx::Error> {
    const REHINT_DAYS: i64 = 1;
    const YEAR_DAYS: f32 = 365.25;

    let rows: Vec<StateRow> = sqlx::query_as(
        "SELECT concept_id, learner_confidence, last_reinforced_at, last_evaluated_at, updated_at, next_decay_at
         FROM learner_concept_states WHERE learner_id = ?",
    )
    .bind(learner_id.to_string())
    .fetch_all(pool)
    .await?;

    let mut decayed = 0usize;
    for (concept_id, confidence, reinforced, evaluated, updated, next_decay) in rows {
        if next_decay.as_deref().map(parse_dt).is_some_and(|hint| hint > now) {
            continue;
        }
        let evaluated_at = parse_dt(&evaluated);
        let anchor = reinforced.as_deref().map(parse_dt).unwrap_or(evaluated_at);
        let grace_end = anchor + chrono::Duration::days((grace_years * YEAR_DAYS) as i64);
        if now <= grace_end {
            // In grace: ensure a recheck hint, never touch confidence.
            if next_decay.is_none() {
                sqlx::query(
                    "UPDATE learner_concept_states SET next_decay_at = ?, updated_at = ? WHERE learner_id = ? AND concept_id = ?",
                )
                .bind(grace_end.to_rfc3339())
                .bind(now.to_rfc3339())
                .bind(learner_id.to_string())
                .bind(&concept_id)
                .execute(pool)
                .await?;
            }
            continue;
        }
        // Tolerance (not f32::EPSILON): sub-second recomputation drifts ~1e-9
        // while a day of half-life decay moves ~1e-4, so rapid re-passes skip
        // without stalling real maintenance intervals.
        let since = parse_dt(&updated).max(grace_end);
        let elapsed_years = (now - since).num_seconds().max(0) as f32 / (YEAR_DAYS * 24.0 * 3600.0);
        let fresh = (confidence * 0.5_f32.powf(elapsed_years / half_life_years)).clamp(0.0, 1.0);
        if (fresh - confidence).abs() <= 1e-6 {
            continue;
        }
        let rehint = now + chrono::Duration::days(REHINT_DAYS);
        sqlx::query(
            "UPDATE learner_concept_states SET learner_confidence = ?, next_decay_at = ?, updated_at = ? WHERE learner_id = ? AND concept_id = ?",
        )
        .bind(fresh)
        .bind(rehint.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(learner_id.to_string())
        .bind(&concept_id)
        .execute(pool)
        .await?;
        decayed += 1;
    }
    Ok(decayed)
}
