use domain::{ConceptNode, ConceptRelation, LearnerObservation};
use infrastructure::{graph_repo, learner_repo, observation_repo, proposal_repo};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

/// Single node in a neighborhood response, with learner health attached.
///
/// `learner_confidence` is `None` when the learner has no state for the concept
/// (unseen). `is_healthy` / `is_review_eligible` derive from
/// `HEALTHY_THRESHOLD = 0.95` and are `None` when confidence is `None`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeighborhoodNode {
    pub concept: ConceptNode,
    pub learner_confidence: Option<f32>,
    pub is_healthy: Option<bool>,
    pub is_review_eligible: Option<bool>,
}

/// Bounded neighborhood: nodes plus edges among those nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Neighborhood {
    pub nodes: Vec<NeighborhoodNode>,
    pub edges: Vec<ConceptRelation>,
}

pub struct GraphService {
    pool: Arc<SqlitePool>,
}

impl GraphService {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    pub async fn get_concept(&self, id: Uuid) -> Result<Option<ConceptNode>, anyhow::Error> {
        Ok(graph_repo::get_concept(&self.pool, id).await?)
    }

    pub async fn get_weak_dependencies(
        &self,
        id: Uuid,
        threshold: f32,
        depth: u8,
    ) -> Result<Vec<(ConceptNode, f32)>, anyhow::Error> {
        Ok(graph_repo::get_weak_dependencies(&self.pool, id, threshold, depth).await?)
    }

    pub async fn find_concepts(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<ConceptNode>, anyhow::Error> {
        Ok(graph_repo::find_concepts(&self.pool, query, limit).await?)
    }

    pub async fn get_related_concepts(
        &self,
        id: Uuid,
        limit: i64,
    ) -> Result<Vec<ConceptRelation>, anyhow::Error> {
        Ok(graph_repo::get_related_concepts(&self.pool, id, limit).await?)
    }

    /// Bounded neighborhood for graph inspection (Brick G1).
    ///
    /// Returns `None` when the root concept does not exist (API maps to 404).
    /// Depth defaults to 3 and clamps to ≤5; limit defaults to 50 and clamps
    /// to [1, 100]. Learner scoping comes from the server context, never from
    /// client-supplied identity.
    pub async fn get_neighborhood(
        &self,
        concept_id: Uuid,
        learner_id: Uuid,
        depth: Option<u8>,
        limit: Option<i64>,
    ) -> Result<Option<Neighborhood>, anyhow::Error> {
        let depth = domain::bounded_depth(depth);
        let limit = limit.unwrap_or(50).clamp(1, 100);
        let Some((nodes, edges)) =
            graph_repo::get_neighborhood(&self.pool, learner_id, concept_id, depth, limit).await?
        else {
            return Ok(None);
        };
        Ok(Some(Neighborhood {
            nodes: nodes
                .into_iter()
                .map(|(concept, learner_confidence)| {
                    let (is_healthy, is_review_eligible) = match learner_confidence {
                        Some(c) => {
                            let healthy = c >= domain::HEALTHY_THRESHOLD;
                            (Some(healthy), Some(!healthy))
                        }
                        None => (None, None),
                    };
                    NeighborhoodNode { concept, learner_confidence, is_healthy, is_review_eligible }
                })
                .collect(),
            edges,
        }))
    }

    pub async fn create_concept(&self, node: &ConceptNode) -> Result<ConceptNode, anyhow::Error> {
        Ok(graph_repo::create_concept(&self.pool, node).await?)
    }

    pub async fn update_learner_confidence(
        &self,
        learner_id: Uuid,
        concept_id: Uuid,
        confidence: f32,
    ) -> Result<(), anyhow::Error> {
        Ok(learner_repo::update_learner_confidence(&self.pool, learner_id, concept_id, confidence)
            .await?)
    }

    pub async fn log_observation(
        &self,
        observation: &LearnerObservation,
    ) -> Result<(), anyhow::Error> {
        Ok(observation_repo::log_observation(&self.pool, observation).await?)
    }

    pub async fn propose_concept(&self, node: &ConceptNode) -> Result<(), anyhow::Error> {
        Ok(proposal_repo::propose_concept(&self.pool, node).await?)
    }

    pub async fn propose_relation(
        &self,
        from_id: Uuid,
        to_id: Uuid,
        relation_type: &str,
        reason: &str,
    ) -> Result<(), anyhow::Error> {
        Ok(proposal_repo::propose_relation(&self.pool, from_id, to_id, relation_type, reason)
            .await?)
    }
}
