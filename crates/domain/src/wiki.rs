//! Personal Knowledge Wiki projection (M8, data_models §21).
//!
//! A persistent human-readable projection of the learner graph, cached in
//! SQLite and maintained with low frequency. The graph stays authoritative;
//! pages are derived artifacts that go stale on mutation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Learner confidence at which a concept earns wiki generation.
pub const WIKI_MASTERY_THRESHOLD: f32 = 0.70;

/// Minimum interval between regenerations of the same page.
pub const WIKI_MIN_REGENERATION_INTERVAL_HOURS: i64 = 24;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrerequisiteAnchor {
    pub concept_id: Uuid,
    pub name: String,
    pub learner_confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedAnchor {
    pub concept_id: Uuid,
    pub name: String,
    pub relation_type: crate::relation::RelationType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptWikiPage {
    pub id: Uuid,
    pub learner_id: Uuid,
    pub concept_id: Uuid,
    pub title: String,
    pub summary: String,
    pub personalized_content: String,
    pub known_prerequisites: Vec<PrerequisiteAnchor>,
    pub related_concepts: Vec<RelatedAnchor>,
    pub learner_confidence_at_generation: f32,
    pub version: u32,
    pub is_stale: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ConceptWikiPage {
    /// Validate domain invariants per data_models §21.
    pub fn validate(&self) -> Result<(), crate::errors::DomainError> {
        if self.title.trim().is_empty() {
            return Err(crate::errors::DomainError::Validation("title is non-empty".into()));
        }
        if self.summary.trim().is_empty() {
            return Err(crate::errors::DomainError::Validation("summary is non-empty".into()));
        }
        if self.personalized_content.trim().is_empty() {
            return Err(crate::errors::DomainError::Validation(
                "personalized_content is non-empty".into(),
            ));
        }
        crate::confidence::validate_confidence(self.learner_confidence_at_generation)?;
        if self.version == 0 {
            return Err(crate::errors::DomainError::Validation("version starts at 1".into()));
        }
        for prereq in &self.known_prerequisites {
            if prereq.name.trim().is_empty() {
                return Err(crate::errors::DomainError::Validation(
                    "prerequisite names are non-empty".into(),
                ));
            }
            crate::confidence::validate_confidence(prereq.learner_confidence)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page() -> ConceptWikiPage {
        ConceptWikiPage {
            id: Uuid::new_v4(),
            learner_id: Uuid::new_v4(),
            concept_id: Uuid::new_v4(),
            title: "Prime Number".into(),
            summary: "Numbers with exactly two factors.".into(),
            personalized_content: "You already know factors well, so ...".into(),
            known_prerequisites: vec![PrerequisiteAnchor {
                concept_id: Uuid::new_v4(),
                name: "Factor".into(),
                learner_confidence: 0.98,
            }],
            related_concepts: vec![],
            learner_confidence_at_generation: 0.98,
            version: 1,
            is_stale: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn valid_page_passes() {
        assert!(page().validate().is_ok());
    }

    #[test]
    fn rejects_empty_content_and_bad_confidence() {
        let mut p = page();
        p.title = "  ".into();
        assert!(p.validate().is_err());
        let mut p = page();
        p.learner_confidence_at_generation = 1.5;
        assert!(p.validate().is_err());
        let mut p = page();
        p.version = 0;
        assert!(p.validate().is_err());
    }

    #[test]
    fn serializes_anchors_for_storage() {
        let v = serde_json::to_value(page()).unwrap();
        assert_eq!(v["known_prerequisites"][0]["name"], "Factor");
        assert_eq!(v["is_stale"], false);
    }
}
