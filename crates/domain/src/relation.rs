use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationType {
    Semantic,
    Dependency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptRelation {
    pub id: Uuid,
    pub from_concept_id: Uuid,
    pub to_concept_id: Uuid,
    pub relation_type: RelationType,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ConceptRelation {
    pub fn validate(&self) -> Result<(), crate::errors::DomainError> {
        if self.from_concept_id == self.to_concept_id {
            return Err(crate::errors::DomainError::Validation(
                "relation cannot be self-referential".into(),
            ));
        }
        Ok(())
    }
}
