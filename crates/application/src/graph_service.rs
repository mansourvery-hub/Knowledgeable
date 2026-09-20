use domain::{ConceptAnnotation, ConceptNode, ConceptRelation, LearnerObservation};
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

/// Mastered concept for the wiki browser (Phase 5a, W1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasteredConcept {
    pub concept: ConceptNode,
    pub learner_confidence: f32,
    /// Wiki page state: `None` = no page yet, `Some(true)` = stale,
    /// `Some(false)` = fresh. The route maps this to `none`/`stale`/`ready`.
    pub wiki_stale: Option<bool>,
}

/// Bounded mastered-concepts list with a truncation flag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasteredList {
    pub items: Vec<MasteredConcept>,
    pub truncated: bool,
}

/// Hard cap for the wiki browser list (Phase 5a, W1). The client filters
/// names locally and falls back to concept search when truncated, so the
/// backend stays a single bounded query.
pub const MAX_MASTERED_LIST_LIMIT: i64 = 200;

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

    /// Direct dependency relations of a concept (M4 `get_dependencies` tool).
    ///
    /// Direct-only by design: transitive prerequisite inspection belongs to
    /// `get_neighborhood` (bounded CTE) and health-filtered repair planning
    /// to `get_weak_dependencies`. The tool contract accepts an optional
    /// `depth` for forward compatibility; values are clamped but currently
    /// only depth >= 1 returns rows (depth 0 yields empty).
    pub async fn get_dependencies(
        &self,
        id: Uuid,
        depth: Option<u8>,
    ) -> Result<Vec<ConceptRelation>, anyhow::Error> {
        let depth = domain::bounded_depth(depth);
        if depth == 0 {
            return Ok(Vec::new());
        }
        Ok(graph_repo::get_dependencies(&self.pool, id).await?)
    }

    /// M7 turn annotation (Brick A2).
    ///
    /// Matches the completed turn text against active concept names and
    /// attaches learner confidence + badge status. Deterministic and bounded:
    /// at most `MAX_ANNOTATIONS_PER_TURN` items in first-mention order, so the
    /// SSE frame stays small. Empty text short-circuits without touching the DB.
    pub async fn annotate_turn(
        &self,
        learner_id: Uuid,
        text: &str,
        limit: Option<usize>,
    ) -> Result<Vec<ConceptAnnotation>, anyhow::Error> {
        let limit = limit.unwrap_or(domain::MAX_ANNOTATIONS_PER_TURN);
        if limit == 0 || text.trim().is_empty() {
            return Ok(Vec::new());
        }
        let candidates = graph_repo::list_annotatable_concepts(&self.pool, learner_id, 500).await?;
        let mut by_id = std::collections::HashMap::with_capacity(candidates.len());
        let mut pairs = Vec::with_capacity(candidates.len());
        for (concept, confidence) in candidates {
            pairs.push((concept.id, concept.canonical_name.clone()));
            by_id.insert(concept.id, (concept.canonical_name, confidence));
        }
        Ok(domain::match_mentions(text, &pairs, limit)
            .into_iter()
            .filter_map(|id| {
                by_id.remove(&id).map(|(name, learner_confidence)| ConceptAnnotation {
                    concept_id: id,
                    name,
                    learner_confidence,
                    status: domain::status_for(learner_confidence),
                })
            })
            .collect())
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

    pub async fn apply_observation(
        &self,
        observation: &LearnerObservation,
    ) -> Result<observation_repo::AppliedObservation, anyhow::Error> {
        let applied = observation_repo::apply_observation(&self.pool, observation).await?;
        // Evidence moves confidence: cached wiki for the concept goes stale.
        if let Some(concept_id) = observation.concept_id {
            if let Err(e) = infrastructure::wiki_repo::mark_stale(
                &self.pool,
                observation.learner_id,
                concept_id,
            )
            .await
            {
                tracing::warn!(error = %e, "wiki stale flag failed after observation");
            }
        }
        Ok(applied)
    }

    pub async fn propose_concept(
        &self,
        learner_id: Uuid,
        conversation_id: Uuid,
        node: &ConceptNode,
    ) -> Result<(), anyhow::Error> {
        // M5 admission gate: the domain owns validation. `ConceptNode::validate`
        // rejects empty names/statements and, for Active nodes, world
        // confidence below WORLD_CONFIDENCE_MIN (0.80). Rejected proposals
        // never reach the candidates table.
        node.validate().map_err(|e| anyhow::anyhow!("proposal rejected: {e}"))?;
        Ok(proposal_repo::propose_concept(&self.pool, learner_id, conversation_id, node).await?)
    }

    pub async fn propose_relation(
        &self,
        learner_id: Uuid,
        conversation_id: Uuid,
        from: domain::ConceptRef,
        to: domain::ConceptRef,
        relation_type: &str,
        reason: &str,
    ) -> Result<Uuid, anyhow::Error> {
        // M5 relation validation gate (data_models §12 + §15 rules).
        if reason.trim().is_empty() {
            return Err(anyhow::anyhow!("proposal rejected: reason is required"));
        }
        let relation_type = match relation_type.trim() {
            "semantic" => domain::RelationType::Semantic,
            "dependency" => domain::RelationType::Dependency,
            other => {
                return Err(anyhow::anyhow!(
                    "proposal rejected: relation_type must be semantic|dependency, got {other:?}"
                ));
            }
        };
        if Self::same_endpoint(&from, &to) {
            return Err(anyhow::anyhow!("proposal rejected: relation cannot be self-referential"));
        }
        self.ensure_endpoint(&from).await?;
        self.ensure_endpoint(&to).await?;
        Ok(proposal_repo::propose_relation(
            &self.pool,
            learner_id,
            conversation_id,
            &from,
            &to,
            relation_type,
            reason.trim(),
        )
        .await?)
    }

    fn same_endpoint(a: &domain::ConceptRef, b: &domain::ConceptRef) -> bool {
        match (a, b) {
            (
                domain::ConceptRef::Existing { concept_id: x },
                domain::ConceptRef::Existing { concept_id: y },
            )
            | (
                domain::ConceptRef::Candidate { candidate_id: x },
                domain::ConceptRef::Candidate { candidate_id: y },
            ) => x == y,
            _ => false,
        }
    }

    async fn ensure_endpoint(&self, endpoint: &domain::ConceptRef) -> Result<(), anyhow::Error> {
        let usable = match endpoint {
            domain::ConceptRef::Existing { concept_id } => {
                proposal_repo::concept_exists(&self.pool, *concept_id).await?
            }
            domain::ConceptRef::Candidate { candidate_id } => {
                proposal_repo::candidate_usable(&self.pool, *candidate_id).await?
            }
        };
        if !usable {
            return Err(anyhow::anyhow!("proposal rejected: relation endpoint does not resolve"));
        }
        Ok(())
    }

    /// Server-scoped learner identity for tool execution (single-user local
    /// mode). Ensures the row exists so candidate FKs never dangle.
    pub async fn ensure_learner(&self) -> Result<Uuid, anyhow::Error> {
        Ok(crate::conversation_service::ensure_default_learner(&self.pool).await?)
    }

    /// Mastered concepts for the wiki browser (Phase 5a, W1): active
    /// concepts at or above the shared known threshold (the wiki lists what
    /// counts as known, matching chat badges), weakest-first.
    /// Fetches one extra row to set `truncated` without a second query.
    pub async fn list_mastered_concepts(
        &self,
        learner_id: Uuid,
        limit: i64,
    ) -> Result<MasteredList, anyhow::Error> {
        let limit = limit.clamp(1, MAX_MASTERED_LIST_LIMIT);
        let rows = graph_repo::list_mastered_concepts(
            &self.pool,
            learner_id,
            domain::ANNOTATION_KNOWN_THRESHOLD,
            limit + 1,
        )
        .await?;
        let truncated = rows.len() as i64 > limit;
        let items = rows
            .into_iter()
            .take(limit as usize)
            .map(|(concept, learner_confidence, wiki_stale)| MasteredConcept {
                concept,
                learner_confidence,
                wiki_stale,
            })
            .collect();
        Ok(MasteredList { items, truncated })
    }

    /// Admit a pending concept candidate: authoritative node + accepted
    /// verdict + audit row commit atomically (see `proposal_repo`).
    pub async fn admit_concept_candidate(
        &self,
        learner_id: Uuid,
        candidate_id: Uuid,
    ) -> Result<Uuid, anyhow::Error> {
        let concept_id =
            proposal_repo::admit_concept_candidate(&self.pool, learner_id, candidate_id).await?;
        // The new node changes the neighborhood: cached wiki goes stale.
        // Auxiliary: log but never fail the admission over the flag.
        if let Err(e) =
            infrastructure::wiki_repo::mark_stale(&self.pool, learner_id, concept_id).await
        {
            tracing::warn!(error = %e, "wiki stale flag failed after admission");
        }
        Ok(concept_id)
    }
}
