use domain::ConceptNode;
use sqlx::SqlitePool;
use uuid::Uuid;

/// Failure modes for candidate admission. `GateRejected` persists the
/// rejection (committed) so the decision is auditable; all other writes in a
/// failed admission roll back together.
#[derive(Debug, thiserror::Error)]
pub enum AdmitCandidateError {
    #[error("candidate not found")]
    NotFound,
    #[error("candidate already decided: {0}")]
    AlreadyDecided(String),
    #[error("candidate rejected by admission gate: {0}")]
    GateRejected(String),
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
}

/// Stored candidate row needed for admission.
type CandidateRow = (String, String, String, Option<String>, f32, String, String, String);

pub async fn propose_concept(
    pool: &SqlitePool,
    learner_id: Uuid,
    conversation_id: Uuid,
    node: &ConceptNode,
) -> Result<(), sqlx::Error> {
    // `status` defaults to 'pending'; `created_at` defaults to now.
    sqlx::query(
        "INSERT INTO concept_candidates (id, learner_id, conversation_id, canonical_name, canonical_statement, learner_statement, world_confidence)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(node.id.to_string())
    .bind(learner_id.to_string())
    .bind(conversation_id.to_string())
    .bind(&node.canonical_name)
    .bind(&node.canonical_statement)
    .bind(&node.learner_statement)
    .bind(node.world_confidence)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn propose_relation(
    pool: &SqlitePool,
    learner_id: Uuid,
    conversation_id: Uuid,
    from: &domain::ConceptRef,
    to: &domain::ConceptRef,
    relation_type: domain::RelationType,
    reason: &str,
) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::new_v4();
    let rel = match relation_type {
        domain::RelationType::Semantic => "semantic",
        domain::RelationType::Dependency => "dependency",
    };
    let (from_concept, from_candidate) = match from {
        domain::ConceptRef::Existing { concept_id } => (Some(concept_id.to_string()), None),
        domain::ConceptRef::Candidate { candidate_id } => (None, Some(candidate_id.to_string())),
    };
    let (to_concept, to_candidate) = match to {
        domain::ConceptRef::Existing { concept_id } => (Some(concept_id.to_string()), None),
        domain::ConceptRef::Candidate { candidate_id } => (None, Some(candidate_id.to_string())),
    };
    // `status` defaults to 'pending'; `created_at` defaults to now.
    sqlx::query(
        "INSERT INTO relation_candidates (id, learner_id, conversation_id, from_concept_id, from_candidate_id, to_concept_id, to_candidate_id, relation_type, reason)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(learner_id.to_string())
    .bind(conversation_id.to_string())
    .bind(from_concept)
    .bind(from_candidate)
    .bind(to_concept)
    .bind(to_candidate)
    .bind(rel)
    .bind(reason)
    .execute(pool)
    .await?;
    Ok(id)
}
/// Resolution helpers for relation validation: authoritative nodes must
/// exist; candidates must exist and not be rejected (rejected rows can never
/// resolve at admission).
pub async fn concept_exists(pool: &SqlitePool, id: Uuid) -> Result<bool, sqlx::Error> {
    let row: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM concept_nodes WHERE id = ? LIMIT 1")
        .bind(id.to_string())
        .fetch_optional(pool)
        .await?;
    Ok(row.is_some())
}

pub async fn candidate_usable(pool: &SqlitePool, id: Uuid) -> Result<bool, sqlx::Error> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT status FROM concept_candidates WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(pool)
            .await?;
    Ok(matches!(row, Some((status,)) if status != "rejected"))
}

/// Admit a pending concept candidate (M5 atomic commit).
///
/// One SQLite transaction: re-check the admission gate against the stored
/// row, insert the authoritative `concept_nodes` row (reusing the candidate
/// id for lineage), flip the candidate to `accepted`, and append the
/// `graph_mutations` audit row. Any failure rolls back all three writes.
/// Gate rejections are the exception: they commit a `rejected` verdict (with
/// reason) and return [`AdmitCandidateError::GateRejected`] with no node and
/// no audit row.
pub async fn admit_concept_candidate(
    pool: &SqlitePool,
    learner_id: Uuid,
    candidate_id: Uuid,
) -> Result<Uuid, AdmitCandidateError> {
    let mut tx = pool.begin().await?;
    let row: Option<CandidateRow> = sqlx::query_as(
        "SELECT id, canonical_name, canonical_statement, learner_statement, world_confidence, status, conversation_id, created_at
         FROM concept_candidates WHERE id = ? AND learner_id = ?",
    )
    .bind(candidate_id.to_string())
    .bind(learner_id.to_string())
    .fetch_optional(&mut *tx)
    .await?;
    let (id, name, statement, learner_statement, confidence, status, convo_id, _) =
        row.ok_or(AdmitCandidateError::NotFound)?;
    if status != "pending" {
        return Err(AdmitCandidateError::AlreadyDecided(status));
    }

    let gate_ok = !name.trim().is_empty()
        && !statement.trim().is_empty()
        && confidence >= domain::WORLD_CONFIDENCE_MIN;
    if !gate_ok {
        let reason = if name.trim().is_empty() || statement.trim().is_empty() {
            "empty canonical name or statement".to_string()
        } else {
            format!(
                "world_confidence {confidence} below admission minimum {}",
                domain::WORLD_CONFIDENCE_MIN
            )
        };
        sqlx::query(
            "UPDATE concept_candidates SET status = 'rejected', rejection_reason = ?, evaluated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ?",
        )
        .bind(&reason)
        .bind(&id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        return Err(AdmitCandidateError::GateRejected(reason));
    }

    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO concept_nodes (id, canonical_name, canonical_statement, learner_statement, world_confidence, status, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, 'active', ?, ?)",
    )
    .bind(&id)
    .bind(&name)
    .bind(&statement)
    .bind(&learner_statement)
    .bind(confidence)
    .bind(&now)
    .bind(&now)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE concept_candidates SET status = 'accepted', evaluated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ?",
    )
    .bind(&id)
    .execute(&mut *tx)
    .await?;
    let mutation_id = Uuid::new_v4();
    let payload = serde_json::json!({
        "kind": "concept_admission",
        "candidate_id": id,
        "concept_id": id,
        "canonical_name": name,
        "world_confidence": confidence,
    })
    .to_string();
    sqlx::query(
        "INSERT INTO graph_mutations (id, learner_id, conversation_id, payload) VALUES (?, ?, ?, ?)",
    )
    .bind(mutation_id.to_string())
    .bind(learner_id.to_string())
    .bind(&convo_id)
    .bind(&payload)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(id.parse().unwrap())
}
