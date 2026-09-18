import { bfsDepths, layoutGraph, toViewBox } from '../graphLayout';
import type { ConceptRelation, NeighborhoodNode } from '../graphTypes';

const A = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
const B = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb';
const C = 'cccccccc-cccc-cccc-cccc-cccccccccccc';
const D = 'dddddddd-dddd-dddd-dddd-dddddddddddd';

function node(id: string, name: string, learnerConfidence: number | null): NeighborhoodNode {
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
    is_healthy: learnerConfidence == null ? null : learnerConfidence >= 0.95,
    is_review_eligible: learnerConfidence == null ? null : learnerConfidence < 0.95,
  };
}

function edge(id: string, from: string, to: string, relation: 'Semantic' | 'Dependency'): ConceptRelation {
  return {
    id,
    from_concept_id: from,
    to_concept_id: to,
    relation_type: relation,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
  };
}

const NODES = [
  node(A, 'Alpha', 0.98),
  node(B, 'Beta', 0.3),
  node(C, 'Gamma', null),
  node(D, 'Delta', 0.9),
];
const EDGES = [
  edge('e1', A, B, 'Dependency'),
  edge('e2', B, C, 'Dependency'),
  edge('e3', A, D, 'Semantic'),
];

describe('graphLayout', () => {
  it('computes BFS depths from the root', () => {
    const depths = bfsDepths(A, new Set([A, B, C, D]), EDGES);
    expect(depths.get(A)).toBe(0);
    expect(depths.get(B)).toBe(1);
    expect(depths.get(D)).toBe(1);
    expect(depths.get(C)).toBe(2);
  });

  it('places every node exactly once with the root centered', () => {
    const layout = layoutGraph(A, NODES, EDGES);
    expect(layout.nodes).toHaveLength(4);
    expect(new Set(layout.nodes.map((n) => n.id)).size).toBe(4);
    const root = layout.nodes.find((n) => n.id === A);
    expect(root).toMatchObject({ x: 0, y: 0, depth: 0, status: 'healthy' });
    const beta = layout.nodes.find((n) => n.id === B);
    expect(beta?.depth).toBe(1);
    expect(beta?.status).toBe('review');
    expect(layout.nodes.find((n) => n.id === C)).toMatchObject({ depth: 2, status: 'unseen' });
    // Ring siblings share a radius and spread apart.
    const ring1 = layout.nodes.filter((n) => n.depth === 1);
    expect(ring1).toHaveLength(2);
    const radii = new Set(ring1.map((n) => Math.round(Math.hypot(n.x, n.y))));
    expect(radii.size).toBe(1);
    expect(Math.hypot(ring1[0].x - ring1[1].x, ring1[0].y - ring1[1].y)).toBeGreaterThan(0);
  });

  it('drops dangling edges and sizes the viewBox to fit', () => {
    const layout = layoutGraph(A, NODES, [
      ...EDGES,
      edge('ghost', A, 'missing-node', 'Dependency'),
    ]);
    expect(layout.edges).toHaveLength(3);
    expect(layout.edges.every((e) => e.fromId !== 'missing-node')).toBe(true);
    for (const n of layout.nodes) {
      const view = toViewBox(n.x, n.y, layout.width);
      expect(view.x).toBeGreaterThanOrEqual(0);
      expect(view.y).toBeGreaterThanOrEqual(0);
      expect(view.x).toBeLessThanOrEqual(layout.width);
      expect(view.y).toBeLessThanOrEqual(layout.height);
    }
  });

  it('parks unreachable nodes on an outer ring instead of dropping them', () => {
    const layout = layoutGraph(A, NODES, []);
    expect(layout.nodes).toHaveLength(4);
    expect(layout.edges).toHaveLength(0);
    expect(layout.nodes.find((n) => n.id === A)?.depth).toBe(0);
    expect(
      layout.nodes.filter((n) => n.id !== A).every((n) => n.depth === 1),
    ).toBe(true);
  });
});
