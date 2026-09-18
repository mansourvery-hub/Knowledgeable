/**
 * Deterministic radial layout for the T23 graph rehaul (no dependencies).
 *
 * BFS depths from the root concept become concentric rings: the root sits at
 * the center, each depth ring spreads its nodes evenly. Edges that point
 * outside the visited closure (dangling after review filtering) are dropped
 * so the renderer never draws dangling endpoints.
 */
import type { ConceptRelation, NeighborhoodNode } from './graphTypes';

export interface PlacedNode {
  id: string;
  name: string;
  x: number;
  y: number;
  depth: number;
  status: 'healthy' | 'review' | 'unseen';
  learnerConfidence: number | null;
}

export interface PlacedEdge {
  id: string;
  fromId: string;
  toId: string;
  relation: 'Semantic' | 'Dependency';
}

export interface GraphLayout {
  nodes: PlacedNode[];
  edges: PlacedEdge[];
  width: number;
  height: number;
}

export const RING_SPACING = 150;
export const NODE_RADIUS = 20;
const CANVAS_PADDING = 70;

function healthOf(learnerConfidence: number | null | undefined): PlacedNode['status'] {
  if (learnerConfidence == null || Number.isNaN(learnerConfidence)) {
    return 'unseen';
  }
  return learnerConfidence >= 0.95 ? 'healthy' : 'review';
}

/** Undirected BFS depths from the root over the visible edge set. */
export function bfsDepths(
  rootId: string,
  nodeIds: Set<string>,
  edges: ConceptRelation[],
): Map<string, number> {
  const adjacency = new Map<string, string[]>();
  const link = (a: string, b: string) => {
    if (nodeIds.has(a) && nodeIds.has(b)) {
      if (!adjacency.has(a)) {
        adjacency.set(a, []);
      }
      adjacency.get(a)?.push(b);
    }
  };
  for (const edge of edges) {
    link(edge.from_concept_id, edge.to_concept_id);
    link(edge.to_concept_id, edge.from_concept_id);
  }
  const depths = new Map<string, number>([[rootId, 0]]);
  const queue = [rootId];
  while (queue.length > 0) {
    const current = queue.shift() as string;
    for (const next of adjacency.get(current) ?? []) {
      if (!depths.has(next)) {
        depths.set(next, (depths.get(current) ?? 0) + 1);
        queue.push(next);
      }
    }
  }
  return depths;
}

export function layoutGraph(
  rootId: string,
  nodes: NeighborhoodNode[],
  edges: ConceptRelation[],
): GraphLayout {
  const byId = new Map(nodes.map((n) => [n.concept.id, n]));
  const nodeIds = new Set(byId.keys());
  const depths = bfsDepths(rootId, nodeIds, edges);

  // Unreached nodes (possible after filtering) form an outer ring so every
  // visible node is always placed exactly once.
  const maxReached = Math.max(0, ...depths.values());
  const rings = new Map<number, string[]>();
  for (const id of nodeIds) {
    const depth = depths.get(id) ?? maxReached + 1;
    if (!rings.has(depth)) {
      rings.set(depth, []);
    }
    rings.get(depth)?.push(id);
  }
  for (const ring of rings.values()) {
    ring.sort();
  }

  const placed = new Map<string, PlacedNode>();
  for (const [depth, ids] of rings) {
    if (depth === 0) {
      const node = byId.get(ids[0]);
      if (node) {
        placed.set(ids[0], {
          id: ids[0],
          name: node.concept.canonical_name,
          x: 0,
          y: 0,
          depth: 0,
          status: healthOf(node.learner_confidence),
          learnerConfidence: node.learner_confidence,
        });
      }
      continue;
    }
    const radius = depth * RING_SPACING;
    ids.forEach((id, i) => {
      const node = byId.get(id);
      if (!node) {
        return;
      }
      const angle = (2 * Math.PI * i) / ids.length - Math.PI / 2;
      placed.set(id, {
        id,
        name: node.concept.canonical_name,
        x: radius * Math.cos(angle),
        y: radius * Math.sin(angle),
        depth,
        status: healthOf(node.learner_confidence),
        learnerConfidence: node.learner_confidence,
      });
    });
  }

  const placedEdges: PlacedEdge[] = [];
  for (const edge of edges) {
    if (placed.has(edge.from_concept_id) && placed.has(edge.to_concept_id)) {
      placedEdges.push({
        id: edge.id,
        fromId: edge.from_concept_id,
        toId: edge.to_concept_id,
        relation: edge.relation_type,
      });
    }
  }

  const extent =
    [...placed.values()].reduce((m, n) => Math.max(m, Math.abs(n.x), Math.abs(n.y)), 0) +
    CANVAS_PADDING;
  const size = Math.max(2 * extent, 2 * (NODE_RADIUS + CANVAS_PADDING));
  return {
    nodes: [...placed.values()],
    edges: placedEdges,
    width: size,
    height: size,
  };
}

/** Shift centered coordinates into positive viewBox space. */
export function toViewBox(x: number, y: number, size: number): { x: number; y: number } {
  return { x: x + size / 2, y: y + size / 2 };
}
