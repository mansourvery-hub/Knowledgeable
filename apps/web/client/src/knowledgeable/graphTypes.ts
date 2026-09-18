/**
 * Graph neighborhood types for the T9 Graph Explorer (M9).
 *
 * These mirror the Rust backend payloads exactly (serde defaults = snake_case):
 * - `crates/application/src/graph_service.rs` (`Neighborhood`, `NeighborhoodNode`)
 * - `crates/domain/src/concept.rs` (`ConceptNode`, `ConceptStatus`)
 * - `crates/domain/src/relation.rs` (`ConceptRelation`, `RelationType`)
 * - `crates/api/src/routes/neighborhood.rs`
 *   (`GET /v1/graph/neighborhood` + `/api/graph/neighborhood` alias)
 *
 * Keep this file free of React/fetch imports so pure helpers stay unit-testable.
 */

/** Backend `RelationType` serializes as `"Semantic" | "Dependency"`. */
export type RelationType = 'Semantic' | 'Dependency';

/** Backend `ConceptStatus` serializes as `"Active" | "Archived"`. */
export type ConceptStatus = 'Active' | 'Archived';

export interface ConceptNode {
  id: string;
  canonical_name: string;
  canonical_statement: string;
  learner_statement: string | null;
  world_confidence: number;
  status: ConceptStatus;
  created_at: string;
  updated_at: string;
}

export interface NeighborhoodNode {
  concept: ConceptNode;
  learner_confidence: number | null;
  is_healthy: boolean | null;
  is_review_eligible: boolean | null;
}

export interface ConceptRelation {
  id: string;
  from_concept_id: string;
  to_concept_id: string;
  relation_type: RelationType;
  created_at: string;
  updated_at: string;
}

export interface Neighborhood {
  nodes: NeighborhoodNode[];
  edges: ConceptRelation[];
}

/** Frontend-only learner health derived from `learner_confidence`. */
export type ConfidenceStatus = 'healthy' | 'review' | 'unseen';
