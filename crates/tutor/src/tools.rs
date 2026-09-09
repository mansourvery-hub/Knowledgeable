//! Typed tutor tool contracts per `data_models.md -> Tutor Tool Contracts`.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindConceptRequest {
    pub query: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetConceptRequest {
    pub concept_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetDependenciesRequest {
    pub concept_id: Uuid,
    pub depth: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetRelatedConceptsRequest {
    pub concept_id: Uuid,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetLearnerConfidenceRequest {
    pub concept_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetWeakDependenciesRequest {
    pub concept_id: Uuid,
    pub threshold: Option<f32>,
    pub depth: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposeConceptRequest {
    pub canonical_name: String,
    pub canonical_statement: String,
    pub learner_statement: Option<String>,
    pub world_confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposeRelationRequest {
    pub from: ConceptRef,
    pub to: ConceptRef,
    pub relation_type: String, // "semantic" | "dependency"
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConceptRef {
    Existing { concept_id: Uuid },
    Candidate { candidate_id: Uuid },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposeLearnerUpdateRequest {
    pub concept_id: Uuid,
    pub new_learner_confidence: f32,
    pub reason: String,
}
