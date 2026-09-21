/**
 * Pure helpers for the T9 Graph Explorer.
 *
 * No React/fetch/DOM imports: safe for fast Jest unit tests and reuse in any
 * future canvas/SVG renderer.
 */
import type { ConfidenceStatus, ConceptRelation, NeighborhoodNode } from './graphTypes';

/** Mirrors `crates/domain/src/confidence.rs::HEALTHY_THRESHOLD`. */
export const HEALTHY_THRESHOLD = 0.95;

/** Backend depth default/clamp mirrors `domain::bounded_depth`. */
export const DEFAULT_DEPTH = 3;
export const MAX_DEPTH = 5;

/** Backend limit default/clamp mirrors `GraphService::get_neighborhood`. */
export const DEFAULT_LIMIT = 50;
export const MIN_LIMIT = 1;
export const MAX_LIMIT = 100;

export function clampDepth(depth: number | undefined | null): number {
  if (depth == null || Number.isNaN(depth)) {
    return DEFAULT_DEPTH;
  }
  return Math.min(MAX_DEPTH, Math.max(0, Math.floor(depth)));
}

export function clampLimit(limit: number | undefined | null): number {
  if (limit == null || Number.isNaN(limit)) {
    return DEFAULT_LIMIT;
  }
  return Math.min(MAX_LIMIT, Math.max(MIN_LIMIT, Math.floor(limit)));
}

/**
 * Derive frontend health from learner confidence.
 * `null`/`undefined`/`NaN` = unseen (no learner state yet).
 */
export function confidenceStatus(
  learnerConfidence: number | null | undefined,
): ConfidenceStatus {
  if (learnerConfidence == null || Number.isNaN(learnerConfidence)) {
    return 'unseen';
  }
  return learnerConfidence >= HEALTHY_THRESHOLD ? 'healthy' : 'review';
}

export type ConfidenceWord = 'Solid' | 'Building' | 'Not met yet';

/** Learner-facing copy deck wording corresponding to ConfidenceStatus. */
export function confidenceWords(
  learnerConfidence: number | null | undefined,
): ConfidenceWord {
  const status = confidenceStatus(learnerConfidence);
  switch (status) {
    case 'healthy':
      return 'Solid';
    case 'review':
      return 'Building';
    case 'unseen':
      return 'Not met yet';
  }
}

/** True when the learner has state and it is below the healthy threshold. */
export function isReviewEligible(node: NeighborhoodNode): boolean {
  const c = node.learner_confidence;
  if (c == null || Number.isNaN(c)) {
    return false;
  }
  // Prefer the server-derived flag when present; fall back to local threshold.
  if (node.is_review_eligible != null) {
    return node.is_review_eligible;
  }
  return c < HEALTHY_THRESHOLD;
}

/** Review/weak view: only nodes that need learner attention. */
export function filterReviewOnly(nodes: NeighborhoodNode[]): NeighborhoodNode[] {
  return nodes.filter(isReviewEligible);
}

export interface NeighborhoodCounts {
  concepts: number;
  links: number;
  needReview: number;
  healthy: number;
  unseen: number;
}

export function countNeighborhood(
  nodes: NeighborhoodNode[],
  edges: ConceptRelation[],
): NeighborhoodCounts {
  let needReview = 0;
  let healthy = 0;
  let unseen = 0;
  for (const node of nodes) {
    const status = confidenceStatus(node.learner_confidence);
    if (status === 'healthy') {
      healthy += 1;
    } else if (status === 'review') {
      needReview += 1;
    } else {
      unseen += 1;
    }
  }
  return { concepts: nodes.length, links: edges.length, needReview, healthy, unseen };
}

/** `0.9` -> `"90%"`; `null` -> `"unseen"`. */
export function formatConfidence(learnerConfidence: number | null | undefined): string {
  if (learnerConfidence == null || Number.isNaN(learnerConfidence)) {
    return 'unseen';
  }
  return `${Math.round(learnerConfidence * 100)}%`;
}

/** Human label preserving the semantic vs dependency distinction. */
export function edgeLabel(relationType: ConceptRelation['relation_type']): string {
  return relationType === 'Dependency' ? 'depends on' : 'related';
}

/** Stable sort: weakest known confidence first, unseen last, then name. */
export function sortByConfidenceAscending(
  nodes: NeighborhoodNode[],
): NeighborhoodNode[] {
  return [...nodes].sort((a, b) => {
    const ac = a.learner_confidence;
    const bc = b.learner_confidence;
    if (ac == null && bc == null) {
      return a.concept.canonical_name.localeCompare(b.concept.canonical_name);
    }
    if (ac == null) {
      return 1;
    }
    if (bc == null) {
      return -1;
    }
    if (ac !== bc) {
      return ac - bc;
    }
    return a.concept.canonical_name.localeCompare(b.concept.canonical_name);
  });
}
