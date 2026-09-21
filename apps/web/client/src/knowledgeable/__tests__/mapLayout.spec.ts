import type { ConceptRelation, NeighborhoodNode } from '../graphTypes';
import {
  LAYOUT_PADDING,
  RANK_GAP,
  layoutLayered,
  pillLabel,
  pillWidth,
} from '../mapLayout';

const A = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
const B = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb';
const C = 'cccccccc-cccc-cccc-cccc-cccccccccccc';

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

function edge(id: string, from: string, to: string, relation: 'Dependency' | 'Semantic'): ConceptRelation {
  return {
    id,
    from_concept_id: from,
    to_concept_id: to,
    relation_type: relation,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
  };
}

function inBox(px: number, py: number, cx: number, cy: number, w: number, h: number): boolean {
  return Math.abs(px - cx) <= w / 2 && Math.abs(py - cy) <= h / 2;
}

describe('mapLayout', () => {
  it('places prerequisites below the root and dependents above', () => {
    const nodes = [node(A, 'Prime Number', 0.98), node(B, 'Divisibility', 0.85), node(C, 'Factor', 0.3)];
    // A builds on B (B is the prerequisite, one rank lower).
    const layout = layoutLayered(A, nodes, [edge('e1', A, B, 'Dependency'), edge('e2', A, C, 'Semantic')]);
    const byId = new Map(layout.nodes.map((n) => [n.id, n]));
    expect(byId.get(A)?.rank).toBe(0);
    expect(byId.get(B)?.rank).toBe(1);
    expect((byId.get(B)?.y ?? 0)).toBeGreaterThan(byId.get(A)?.y ?? 0);
    expect(byId.get(A)?.y).toBe(LAYOUT_PADDING);
    expect(byId.get(B)?.y).toBe(LAYOUT_PADDING + RANK_GAP);
    expect(byId.get(A)?.focus).toBe(true);
    expect(byId.get(B)?.focus).toBe(false);
  });

  it('is deterministic for the same input', () => {
    const nodes = [node(A, 'Prime Number', 0.98), node(B, 'Divisibility', 0.85), node(C, 'Factor', 0.3)];
    const edges = [edge('e1', A, B, 'Dependency'), edge('e2', B, C, 'Dependency')];
    expect(layoutLayered(A, nodes, edges)).toEqual(layoutLayered(A, nodes, edges));
  });

  it('survives dependency cycles without throwing', () => {
    const nodes = [node(A, 'Alpha', 0.5), node(B, 'Beta', 0.5)];
    const edges = [edge('e1', A, B, 'Dependency'), edge('e2', B, A, 'Dependency')];
    const layout = layoutLayered(A, nodes, edges);
    expect(layout.nodes).toHaveLength(2);
    expect(layout.edges).toHaveLength(2);
  });

  it('gives semantic-only nodes a neighbour rank and orphans a bottom rank', () => {
    const orphan = 'dddddddd-dddd-dddd-dddd-dddddddddddd';
    const nodes = [node(A, 'Alpha', 0.9), node(B, 'Beta', 0.9), node(C, 'Gamma', 0.9), node(orphan, 'Orphan', null)];
    // B reaches A only through a semantic link; the orphan has no links at all.
    const layout = layoutLayered(A, nodes, [edge('e1', A, B, 'Semantic')]);
    const byId = new Map(layout.nodes.map((n) => [n.id, n]));
    expect(byId.get(B)?.rank).toBe(byId.get(A)?.rank);
    const maxPlaced = Math.max(byId.get(A)?.rank ?? 0, byId.get(B)?.rank ?? 0);
    expect(byId.get(orphan)?.rank).toBeGreaterThan(maxPlaced);
    // Unseen confidence maps to the unseen status.
    expect(byId.get(orphan)?.status).toBe('unseen');
  });

  it('routes dependency edges from pill bottom to 3px above the target', () => {
    const nodes = [node(A, 'Prime Number', 0.98), node(B, 'Divisibility', 0.85)];
    const layout = layoutLayered(A, nodes, [edge('e1', A, B, 'Dependency')]);
    const pills = new Map(layout.nodes.map((n) => [n.id, n]));
    expect(layout.edges).toHaveLength(1);
    const [placed] = layout.edges;
    expect(placed.relation).toBe('Dependency');
    expect(placed.d).toMatch(/^M .* C .*$/);
    const from = pills.get(A) as { x: number; y: number; width: number; height: number };
    const to = pills.get(B) as { x: number; y: number; width: number; height: number };
    // Start sits on the dependent's bottom edge; end clears the target box.
    expect(placed.x1).toBe(from.x);
    expect(placed.y1).toBe(from.y + from.height / 2);
    expect(placed.x2).toBe(to.x);
    expect(placed.y2).toBe(to.y - to.height / 2 - 3);
    expect(inBox(placed.x2, placed.y2, to.x, to.y, to.width, to.height)).toBe(false);
    expect(inBox(placed.x2, placed.y2, from.x, from.y, from.width, from.height)).toBe(false);
  });

  it('orders nodes rank-major so Tab order matches the picture', () => {
    const nodes = [node(C, 'Zulu', 0.1), node(A, 'Alpha', 0.9), node(B, 'Mike', 0.5)];
    const layout = layoutLayered(A, nodes, [edge('e1', A, B, 'Dependency')]);
    const ranks = layout.nodes.map((n) => n.rank);
    expect([...ranks].sort((a, b) => a - b)).toEqual(ranks);
    // Weakest-first is the panel list's job; the canvas keeps rank order.
    expect(layout.nodes[0].id).toBe(A);
  });

  it('truncates long labels but keeps the full name', () => {
    expect(pillLabel('Factor')).toBe('Factor');
    const long = 'A very long concept name that exceeds twenty-eight characters';
    const label = pillLabel(long);
    expect(label.endsWith('…')).toBe(true);
    expect(label.length).toBeLessThanOrEqual(28);
    const layout = layoutLayered(A, [node(A, long, 0.5)], []);
    expect(layout.nodes[0].label).toBe(label);
    expect(layout.nodes[0].name).toBe(long);
  });

  it('estimates pill widths with the spec formula and compacts past 30 nodes', () => {
    expect(pillWidth('Factor', false, false)).toBe(96); // clamp floor
    expect(pillWidth('Divisibility', false, false)).toBeCloseTo(44 + 12 * 7.4, 5);
    expect(pillWidth('Divisibility', true, false) - pillWidth('Divisibility', false, false)).toBe(20);
    const many = Array.from({ length: 31 }, (_, i) =>
      node(`id-${i}`, `Concept number ${i} with a long name`, 0.5),
    );
    const layout = layoutLayered('id-0', many, []);
    for (const pill of layout.nodes) {
      expect(pill.width).toBeLessThanOrEqual(140);
    }
  });

  it('floors the stage on the container size when provided', () => {
    const layout = layoutLayered(A, [node(A, 'Alpha', 0.9)], [], { width: 500, height: 400 });
    expect(layout.width).toBeGreaterThanOrEqual(500);
    expect(layout.height).toBeGreaterThanOrEqual(400);
  });

  it('drops edges with endpoints outside the visible set', () => {
    const nodes = [node(A, 'Alpha', 0.9)];
    const layout = layoutLayered(A, nodes, [edge('e1', A, B, 'Dependency')]);
    expect(layout.edges).toHaveLength(0);
  });
});
