import type { NeighborhoodNode } from '../graphTypes';
import {
  HEALTHY_THRESHOLD,
  clampDepth,
  clampLimit,
  confidenceStatus,
  confidenceWords,
  countNeighborhood,
  edgeLabel,
  filterReviewOnly,
  formatConfidence,
  isReviewEligible,
  sortByConfidenceAscending,
} from '../graphUtils';

function node(
  id: string,
  name: string,
  learnerConfidence: number | null,
  isReviewEligibleFlag: boolean | null = null,
): NeighborhoodNode {
  return {
    concept: {
      id,
      canonical_name: name,
      canonical_statement: `${name} statement.`,
      learner_statement: null,
      world_confidence: 1.0,
      status: 'Active',
      created_at: '2026-01-01T00:00:00Z',
      updated_at: '2026-01-01T00:00:00Z',
    },
    learner_confidence: learnerConfidence,
    is_healthy:
      learnerConfidence == null ? null : learnerConfidence >= HEALTHY_THRESHOLD,
    is_review_eligible: isReviewEligibleFlag,
  };
}

describe('graphUtils', () => {
  it('clamps depth to [0,5] with default 3', () => {
    expect(clampDepth(undefined)).toBe(3);
    expect(clampDepth(null)).toBe(3);
    expect(clampDepth(0)).toBe(0);
    expect(clampDepth(2)).toBe(2);
    expect(clampDepth(5)).toBe(5);
    expect(clampDepth(255)).toBe(5);
  });

  it('clamps limit to [1,100] with default 50', () => {
    expect(clampLimit(undefined)).toBe(50);
    expect(clampLimit(0)).toBe(1);
    expect(clampLimit(2)).toBe(2);
    expect(clampLimit(500)).toBe(100);
  });

  it('derives healthy/review/unseen from the 0.95 threshold', () => {
    expect(confidenceStatus(0.98)).toBe('healthy');
    expect(confidenceStatus(0.95)).toBe('healthy');
    expect(confidenceStatus(0.3)).toBe('review');
    expect(confidenceStatus(null)).toBe('unseen');
    expect(confidenceStatus(undefined)).toBe('unseen');
  });

  it('maps confidence values to learner-facing words Solid/Building/Not met yet', () => {
    expect(confidenceWords(0.98)).toBe('Solid');
    expect(confidenceWords(0.95)).toBe('Solid');
    expect(confidenceWords(0.85)).toBe('Building');
    expect(confidenceWords(0.3)).toBe('Building');
    expect(confidenceWords(0)).toBe('Building');
    expect(confidenceWords(null)).toBe('Not met yet');
    expect(confidenceWords(undefined)).toBe('Not met yet');
    expect(confidenceWords(NaN)).toBe('Not met yet');
  });

  it('treats unseen nodes as not review-eligible', () => {
    expect(isReviewEligible(node('a', 'A', null))).toBe(false);
    expect(isReviewEligible(node('b', 'B', 0.3))).toBe(true);
    expect(isReviewEligible(node('c', 'C', 0.98))).toBe(false);
  });

  it('prefers the server-derived review flag when present', () => {
    // Stale local threshold vs authoritative server flag: server wins.
    expect(isReviewEligible(node('a', 'A', 0.3, false))).toBe(false);
    expect(isReviewEligible(node('b', 'B', 0.98, true))).toBe(true);
  });

  it('filters the review/weak view', () => {
    const nodes = [node('a', 'A', 0.98), node('b', 'B', 0.3), node('c', 'C', null)];
    expect(filterReviewOnly(nodes).map((n) => n.concept.id)).toEqual(['b']);
  });

  it('counts concepts, links, and health buckets', () => {
    const nodes = [node('a', 'A', 0.98), node('b', 'B', 0.3), node('c', 'C', null)];
    const counts = countNeighborhood(nodes, []);
    expect(counts).toEqual({ concepts: 3, links: 0, needReview: 1, healthy: 1, unseen: 1 });
  });

  it('formats confidence as percent or unseen', () => {
    expect(formatConfidence(0.9)).toBe('90%');
    expect(formatConfidence(null)).toBe('unseen');
  });

  it('labels the semantic vs dependency distinction', () => {
    expect(edgeLabel('Dependency')).toBe('depends on');
    expect(edgeLabel('Semantic')).toBe('related');
  });

  it('sorts weakest first with unseen last', () => {
    const nodes = [node('c', 'C', null), node('a', 'A', 0.98), node('b', 'B', 0.3)];
    expect(sortByConfidenceAscending(nodes).map((n) => n.concept.id)).toEqual([
      'b',
      'a',
      'c',
    ]);
  });
});
