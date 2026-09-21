/**
 * Deterministic layered layout for the Map panel (SPEC 6.3).
 *
 * Pure: no React/DOM imports, safe for fast Jest unit tests.
 * Direction: what a concept builds on sits below it. A `Dependency` edge
 * `from depends_on to` places `to` one rank lower than `from`; semantic
 * edges never affect rank.
 */
import type { ConceptRelation, NeighborhoodNode } from './graphTypes';
import { confidenceStatus } from './graphUtils';

export type PillStatus = 'healthy' | 'review' | 'unseen';

export interface PlacedPill {
  id: string;
  /** Full concept name (also used for the `title` tooltip). */
  name: string;
  /** Display label: truncated with an ellipsis past `MAX_LABEL` characters. */
  label: string;
  /** Pill centre in stage coordinates. */
  x: number;
  y: number;
  width: number;
  height: number;
  rank: number;
  status: PillStatus;
  /** The queried root concept (larger pill). */
  focus: boolean;
}

export interface PlacedEdge {
  id: string;
  fromId: string;
  toId: string;
  relation: 'Semantic' | 'Dependency';
  /** SVG path data for `svg.k-edges`. */
  d: string;
  /** Path start (dependent side). */
  x1: number;
  y1: number;
  /** Path end (prerequisite side, 3px clear of the target pill). */
  x2: number;
  y2: number;
}

export interface MapLayout {
  /** Rank-major, left-to-right so Tab order matches the picture. */
  nodes: PlacedPill[];
  edges: PlacedEdge[];
  width: number;
  height: number;
}

export const RANK_GAP = 96;
export const PILL_GAP = 16;
export const LAYOUT_PADDING = 40;
export const MAX_LABEL = 28;
export const STANDARD_HEIGHT = 36;
export const FOCUS_HEIGHT = 42;
export const COMPACT_MAX_WIDTH = 140;
/** Arrowhead clearance: dependency paths end this far above the target. */
export const ARROW_CLEARANCE = 3;

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

/** `clamp(96, 44 + name.length * 7.4, 220)` for 13.5px Inter; focus `+ 20`. */
export function pillWidth(name: string, focus: boolean, compact: boolean): number {
  const raw = 44 + name.length * 7.4 + (focus ? 20 : 0);
  // Compact lower bound (64) is a conservative choice; the spec pins only
  // the 140 upper bound. Focus keeps its +20 even when compact.
  return compact ? clamp(raw, 64, COMPACT_MAX_WIDTH) : clamp(raw, 96, 220);
}

/** Names over 28 characters truncate with an ellipsis (mirrors old shortLabel). */
export function pillLabel(name: string): string {
  return name.length > MAX_LABEL ? `${name.slice(0, MAX_LABEL - 1)}…` : name;
}

interface DepEdge {
  from: string;
  to: string;
}

function dependencyEdges(nodes: Set<string>, edges: ConceptRelation[]): DepEdge[] {
  return edges
    .filter((e) => e.relation_type === 'Dependency' && nodes.has(e.from_concept_id) && nodes.has(e.to_concept_id))
    .map((e) => ({ from: e.from_concept_id, to: e.to_concept_id }));
}

/** BFS from the root over dependency edges: prerequisite `rank + 1`, dependent `rank - 1`. */
function seedRanks(rootId: string, nodeIds: Set<string>, deps: DepEdge[]): Map<string, number> {
  const adjacency = new Map<string, Array<{ next: string; delta: number }>>();
  const link = (a: string, b: string, delta: number) => {
    const list = adjacency.get(a) ?? [];
    list.push({ next: b, delta });
    adjacency.set(a, list);
  };
  for (const dep of deps) {
    link(dep.from, dep.to, 1);
    link(dep.to, dep.from, -1);
  }
  const ranks = new Map<string, number>([[rootId, 0]]);
  const queue = [rootId];
  while (queue.length > 0) {
    const current = queue.shift() as string;
    for (const { next, delta } of adjacency.get(current) ?? []) {
      if (!ranks.has(next)) {
        ranks.set(next, (ranks.get(current) ?? 0) + delta);
        queue.push(next);
      }
    }
  }
  // The root may itself be absent from `nodes`; drop it so every rank
  // belongs to a placed pill.
  if (!nodeIds.has(rootId)) {
    ranks.delete(rootId);
  }
  return ranks;
}

/** Enforce `rank(to) >= rank(from) + 1`; the pass cap breaks cycles safely. */
function repairRanks(ranks: Map<string, number>, deps: DepEdge[], passes: number): void {
  for (let pass = 0; pass < passes; pass += 1) {
    let changed = false;
    for (const dep of deps) {
      const fromRank = ranks.get(dep.from);
      const toRank = ranks.get(dep.to);
      if (fromRank == null || toRank == null) {
        continue;
      }
      if (toRank < fromRank + 1) {
        ranks.set(dep.to, fromRank + 1);
        changed = true;
      }
    }
    if (!changed) {
      return;
    }
  }
}

function byName(a: NeighborhoodNode, b: NeighborhoodNode): number {
  return a.concept.canonical_name.localeCompare(b.concept.canonical_name);
}

/**
 * Rank every node: BFS seeds, repair passes, then unreached nodes take a
 * placed semantic neighbour's rank (first in edge order) or a shared extra
 * rank at the bottom. Normalised so the smallest rank is 0.
 */
function assignRanks(
  rootId: string,
  nodes: NeighborhoodNode[],
  edges: ConceptRelation[],
): Map<string, number> {
  const byId = new Map(nodes.map((n) => [n.concept.id, n]));
  const nodeIds = new Set(byId.keys());
  const deps = dependencyEdges(nodeIds, edges);
  const ranks = seedRanks(rootId, nodeIds, deps);
  repairRanks(ranks, deps, Math.max(1, nodes.length));

  const unreached = nodes.filter((n) => !ranks.has(n.concept.id)).sort(byName);
  const semanticOf = (id: string): ConceptRelation[] =>
    edges.filter(
      (e) =>
        e.relation_type === 'Semantic' &&
        nodeIds.has(e.from_concept_id) &&
        nodeIds.has(e.to_concept_id) &&
        (e.from_concept_id === id || e.to_concept_id === id),
    );
  const orphans: NeighborhoodNode[] = [];
  for (const node of unreached) {
    const neighbour = semanticOf(node.concept.id).find((e) => {
      const other = e.from_concept_id === node.concept.id ? e.to_concept_id : e.from_concept_id;
      return ranks.has(other);
    });
    if (neighbour) {
      const other =
        neighbour.from_concept_id === node.concept.id
          ? neighbour.to_concept_id
          : neighbour.from_concept_id;
      ranks.set(node.concept.id, ranks.get(other) as number);
    } else {
      orphans.push(node);
    }
  }
  if (orphans.length > 0) {
    const bottom = ranks.size > 0 ? Math.max(...ranks.values()) + 1 : 0;
    for (const node of orphans) {
      ranks.set(node.concept.id, bottom);
    }
  }

  const min = ranks.size > 0 ? Math.min(...ranks.values()) : 0;
  for (const [id, rank] of ranks) {
    ranks.set(id, rank - min);
  }
  return ranks;
}

/** Rank groups in alphabetical order, ready for barycentre sweeps. */
function groupedByRank(nodes: NeighborhoodNode[], ranks: Map<string, number>): Map<number, NeighborhoodNode[]> {
  const groups = new Map<number, NeighborhoodNode[]>();
  for (const node of nodes) {
    const rank = ranks.get(node.concept.id) ?? 0;
    const group = groups.get(rank) ?? [];
    group.push(node);
    groups.set(rank, group);
  }
  for (const group of groups.values()) {
    group.sort(byName);
  }
  return groups;
}

/**
 * One barycentre sweep: reorder each rank by the mean index of its
 * dependency neighbours in the adjacent rank (ties by name). Nodes with no
 * neighbours there keep their current index, keeping the order stable.
 */
function barycentreSweep(
  groups: Map<number, NeighborhoodNode[]>,
  deps: DepEdge[],
  maxRank: number,
  downward: boolean,
): void {
  const order = downward
    ? Array.from({ length: maxRank }, (_, i) => i + 1)
    : Array.from({ length: maxRank }, (_, i) => maxRank - 1 - i);
  for (const rank of order) {
    const adjacent = downward ? rank - 1 : rank + 1;
    const above = groups.get(adjacent) ?? [];
    const indexOf = new Map(above.map((n, i) => [n.concept.id, i]));
    const group = groups.get(rank) ?? [];
    const currentIndex = new Map(group.map((n, i) => [n.concept.id, i]));
    const neighbours = new Map<string, string[]>();
    for (const dep of deps) {
      const pair: Array<[string, string]> = [
        [dep.from, dep.to],
        [dep.to, dep.from],
      ];
      for (const [a, b] of pair) {
        const list = neighbours.get(a) ?? [];
        list.push(b);
        neighbours.set(a, list);
      }
    }
    group.sort((a, b) => {
      const score = (n: NeighborhoodNode): number => {
        const idx = (neighbours.get(n.concept.id) ?? [])
          .map((id) => indexOf.get(id))
          .filter((i): i is number => i != null);
        if (idx.length === 0) {
          return currentIndex.get(n.concept.id) ?? 0;
        }
        return idx.reduce((s, i) => s + i, 0) / idx.length;
      };
      return score(a) - score(b) || byName(a, b);
    });
  }
}

function routeDependencyEdge(from: PlacedPill, to: PlacedPill, id: string): PlacedEdge {
  const x1 = from.x;
  const y1 = from.y + from.height / 2;
  const x2 = to.x;
  const y2 = to.y - to.height / 2 - ARROW_CLEARANCE;
  const ym = (y1 + y2) / 2;
  return {
    id,
    fromId: from.id,
    toId: to.id,
    relation: 'Dependency',
    d: `M ${x1} ${y1} C ${x1} ${ym}, ${x2} ${ym}, ${x2} ${y2}`,
    x1,
    y1,
    x2,
    y2,
  };
}

/** Clip a centre-to-centre segment to a pill box border. */
function clipToBox(
  cx: number,
  cy: number,
  halfW: number,
  halfH: number,
  tx: number,
  ty: number,
): { x: number; y: number } {
  const dx = tx - cx;
  const dy = ty - cy;
  const t = Math.min(
    dx === 0 ? Infinity : halfW / Math.abs(dx),
    dy === 0 ? Infinity : halfH / Math.abs(dy),
  );
  return { x: cx + dx * t, y: cy + dy * t };
}

function routeSemanticEdge(from: PlacedPill, to: PlacedPill, id: string): PlacedEdge {
  if (from.rank === to.rank) {
    // Shallow arc between the facing pill sides.
    const left = from.x <= to.x ? from : to;
    const right = from.x <= to.x ? to : from;
    const x1 = left.x + left.width / 2;
    const x2 = right.x - right.width / 2;
    const y1 = left.y;
    const y2 = right.y;
    const mx = (x1 + x2) / 2;
    const my = Math.min(y1, y2) - 14;
    return {
      id,
      fromId: from.id,
      toId: to.id,
      relation: 'Semantic',
      d: `M ${x1} ${y1} Q ${mx} ${my}, ${x2} ${y2}`,
      x1,
      y1,
      x2,
      y2,
    };
  }
  const a = clipToBox(from.x, from.y, from.width / 2, from.height / 2, to.x, to.y);
  const b = clipToBox(to.x, to.y, to.width / 2, to.height / 2, from.x, from.y);
  return {
    id,
    fromId: from.id,
    toId: to.id,
    relation: 'Semantic',
    d: `M ${a.x} ${a.y} L ${b.x} ${b.y}`,
    x1: a.x,
    y1: a.y,
    x2: b.x,
    y2: b.y,
  };
}

export interface MinSize {
  width: number;
  height: number;
}

export function layoutLayered(
  rootId: string,
  nodes: NeighborhoodNode[],
  edges: ConceptRelation[],
  minSize: MinSize = { width: 0, height: 0 },
): MapLayout {
  const byId = new Map(nodes.map((n) => [n.concept.id, n]));
  const nodeIds = new Set(byId.keys());
  const ranks = assignRanks(rootId, nodes, edges);
  const deps = dependencyEdges(nodeIds, edges);
  const groups = groupedByRank(nodes, ranks);
  const maxRank = groups.size > 0 ? Math.max(...groups.keys()) : 0;
  barycentreSweep(groups, deps, maxRank, true);
  barycentreSweep(groups, deps, maxRank, false);

  const compact = nodes.length > 30;
  const pills = new Map<string, PlacedPill>();
  const rowWidths = new Map<number, number>();
  for (const [rank, group] of groups) {
    let rowWidth = 0;
    for (const node of group) {
      const focus = node.concept.id === rootId;
      const width = pillWidth(node.concept.canonical_name, focus, compact);
      const height = focus ? FOCUS_HEIGHT : STANDARD_HEIGHT;
      pills.set(node.concept.id, {
        id: node.concept.id,
        name: node.concept.canonical_name,
        label: pillLabel(node.concept.canonical_name),
        x: 0,
        y: LAYOUT_PADDING + rank * RANK_GAP,
        width,
        height,
        rank,
        status: confidenceStatus(node.learner_confidence),
        focus,
      });
      rowWidth += width;
    }
    rowWidth += Math.max(0, group.length - 1) * PILL_GAP;
    rowWidths.set(rank, rowWidth);
  }

  const contentWidth = rowWidths.size > 0 ? Math.max(...rowWidths.values()) : 0;
  const width = Math.max(contentWidth + LAYOUT_PADDING * 2, minSize.width);
  let bottom = 0;
  for (const [rank, group] of groups) {
    const offset = (contentWidth - (rowWidths.get(rank) ?? 0)) / 2;
    let cursor = LAYOUT_PADDING + offset;
    for (const node of group) {
      const pill = pills.get(node.concept.id) as PlacedPill;
      pill.x = cursor + pill.width / 2;
      bottom = Math.max(bottom, pill.y + pill.height / 2);
      cursor += pill.width + PILL_GAP;
    }
  }
  const height = Math.max(bottom + LAYOUT_PADDING, minSize.height);

  const placedEdges: PlacedEdge[] = [];
  for (const edge of edges) {
    const from = pills.get(edge.from_concept_id);
    const to = pills.get(edge.to_concept_id);
    if (!from || !to) {
      continue;
    }
    placedEdges.push(
      edge.relation_type === 'Dependency'
        ? routeDependencyEdge(from, to, edge.id)
        : routeSemanticEdge(from, to, edge.id),
    );
  }

  const ordered = [...pills.values()].sort(
    (a, b) => a.rank - b.rank || a.x - b.x || a.name.localeCompare(b.name),
  );
  return { nodes: ordered, edges: placedEdges, width, height };
}
