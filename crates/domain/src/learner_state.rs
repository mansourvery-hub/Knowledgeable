use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnerConceptState {
    pub learner_id: Uuid,
    pub concept_id: Uuid,
    pub learner_confidence: f32,
    pub last_evaluated_at: DateTime<Utc>,
    pub last_reinforced_at: Option<DateTime<Utc>>,
    pub next_decay_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl LearnerConceptState {
    pub fn validate(&self) -> Result<(), crate::errors::DomainError> {
        crate::confidence::validate_confidence(self.learner_confidence)?;
        Ok(())
    }

    #[must_use]
    pub fn is_healthy(&self) -> bool {
        self.learner_confidence >= crate::confidence::HEALTHY_THRESHOLD
    }

    #[must_use]
    pub fn is_review_eligible(&self) -> bool {
        self.learner_confidence < crate::confidence::HEALTHY_THRESHOLD
    }

    /// Bounded update: clamps to [0,1] and validates delta. Pure domain — no telemetry.
    pub fn apply_delta(&mut self, delta: f32) -> Result<(), crate::errors::DomainError> {
        crate::confidence::validate_delta(delta)?;
        self.learner_confidence = (self.learner_confidence + delta).clamp(0.0, 1.0);
        Ok(())
    }
}

/// Convenience helpers re-exported from former `learner` crate (now domain).
pub fn is_review_eligible(state: &LearnerConceptState) -> bool {
    state.is_review_eligible()
}

pub fn apply_delta(
    state: &mut LearnerConceptState,
    delta: f32,
) -> Result<(), crate::errors::DomainError> {
    state.apply_delta(delta)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObservationType {
    Understands,
    Confusion,
    Misconception,
    RecallFailure,
    ApplicationFailure,
    NewUnderstanding,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnerObservation {
    pub id: Uuid,
    pub learner_id: Uuid,
    pub conversation_id: Uuid,
    pub concept_id: Option<Uuid>,
    pub observation_type: ObservationType,
    pub confidence_delta: Option<f32>,
    pub evidence: String,
    pub created_at: DateTime<Utc>,
}
