use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::confidence::WORLD_CONFIDENCE_MIN;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConceptStatus {
    Active,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptNode {
    pub id: Uuid,
    pub canonical_name: String,
    pub canonical_statement: String,
    pub learner_statement: Option<String>,
    pub world_confidence: f32,
    pub status: ConceptStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ConceptNode {
    /// Validate domain invariants per `data_models.md -> Concept Node`.
    pub fn validate(&self) -> Result<(), crate::errors::DomainError> {
        if self.canonical_name.trim().is_empty() {
            return Err(crate::errors::DomainError::Validation(
                "canonical_name is non-empty".into(),
            ));
        }
        if self.canonical_statement.trim().is_empty() {
            return Err(crate::errors::DomainError::Validation(
                "canonical_statement is non-empty".into(),
            ));
        }
        if !(0.0..=1.0).contains(&self.world_confidence) {
            return Err(crate::errors::DomainError::Validation(
                "world_confidence must be within [0,1]".into(),
            ));
        }
        // Authoritative nodes must satisfy admission gate; candidates may be lower
        // but this check is for authoritative state.
        if self.status == ConceptStatus::Active && self.world_confidence < WORLD_CONFIDENCE_MIN {
            return Err(crate::errors::DomainError::Validation(format!(
                "world_confidence {0} below minimum {1}",
                self.world_confidence, WORLD_CONFIDENCE_MIN
            )));
        }
        Ok(())
    }
}
