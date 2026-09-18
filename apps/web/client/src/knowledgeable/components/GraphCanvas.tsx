/**
 * SVG knowledge-graph renderer for the T23 rehaul (no visualization deps).
 *
 * Positions come from `graphLayout.ts` (deterministic BFS rings); this
 * component only maps them to SVG: health-colored nodes, solid directed
 * arrows for dependencies, dashed lines for semantic links. Fully keyboard
 * operable; the text list in `GraphExplorer` remains the screen-reader
 * fallback alongside these labeled controls.
 */
import type { ConceptRelation, NeighborhoodNode } from '../graphTypes';
import { NODE_RADIUS, layoutGraph, toViewBox } from '../graphLayout';

export interface GraphCanvasProps {
  rootId: string;
  nodes: NeighborhoodNode[];
  edges: ConceptRelation[];
  onSelectConcept?: (conceptId: string) => void;
}

const NODE_FILL: Record<string, string> = {
  healthy: 'fill-emerald-500',
  review: 'fill-amber-500',
  unseen: 'fill-slate-400',
};

const MAX_LABEL = 16;

export function shortLabel(name: string): string {
  return name.length > MAX_LABEL ? `${name.slice(0, MAX_LABEL - 1)}…` : name;
}

export default function GraphCanvas({ rootId, nodes, edges, onSelectConcept }: GraphCanvasProps) {
  const layout = layoutGraph(rootId, nodes, edges);
  if (layout.nodes.length === 0) {
    return null;
  }
  return (
    <svg
      data-testid="graph-canvas"
      viewBox={`0 0 ${layout.width} ${layout.height}`}
      role="img"
      aria-label="Knowledge graph visualization"
      className="h-auto w-full"
    >
      <defs>
        <marker
          id="graph-canvas-arrow"
          viewBox="0 0 10 10"
          refX="9"
          refY="5"
          markerWidth="7"
          markerHeight="7"
          orient="auto-start-reverse"
        >
          <path d="M 0 1 L 9 5 L 0 9 z" className="fill-slate-500" />
        </marker>
      </defs>
      {layout.edges.map((edge) => {
        const from = toViewBox(
          layout.nodes.find((n) => n.id === edge.fromId)?.x ?? 0,
          layout.nodes.find((n) => n.id === edge.fromId)?.y ?? 0,
          layout.width,
        );
        const to = toViewBox(
          layout.nodes.find((n) => n.id === edge.toId)?.x ?? 0,
          layout.nodes.find((n) => n.id === edge.toId)?.y ?? 0,
          layout.width,
        );
        const isDependency = edge.relation === 'Dependency';
        return (
          <line
            key={edge.id}
            data-testid="graph-canvas-edge"
            data-relation={edge.relation}
            x1={from.x}
            y1={from.y}
            x2={to.x}
            y2={to.y}
            className={isDependency ? 'stroke-slate-500' : 'stroke-slate-400'}
            strokeWidth={isDependency ? 2 : 1.5}
            strokeDasharray={isDependency ? undefined : '5 4'}
            markerEnd={isDependency ? 'url(#graph-canvas-arrow)' : undefined}
          />
        );
      })}
      {layout.nodes.map((node) => {
        const pos = toViewBox(node.x, node.y, layout.width);
        const clickable = onSelectConcept != null;
        return (
          <g
            key={node.id}
            data-testid="graph-canvas-node"
            data-concept-id={node.id}
            data-status={node.status}
            transform={`translate(${pos.x},${pos.y})`}
            role={clickable ? 'button' : undefined}
            tabIndex={clickable ? 0 : undefined}
            aria-label={`${node.name}, ${node.status}`}
            className={clickable ? 'cursor-pointer' : undefined}
            onClick={clickable ? () => onSelectConcept?.(node.id) : undefined}
            onKeyDown={
              clickable
                ? (event) => {
                    if (event.key === 'Enter' || event.key === ' ') {
                      event.preventDefault();
                      onSelectConcept?.(node.id);
                    }
                  }
                : undefined
            }
          >
            <title>{node.name}</title>
            <circle
              r={node.depth === 0 ? NODE_RADIUS + 4 : NODE_RADIUS}
              className={`${NODE_FILL[node.status] ?? 'fill-slate-400'} stroke-white dark:stroke-slate-900`}
              strokeWidth={node.depth === 0 ? 3 : 1.5}
            />
            <text
              y={NODE_RADIUS + 16}
              textAnchor="middle"
              className="fill-slate-700 text-xs dark:fill-slate-200"
            >
              {shortLabel(node.name)}
            </text>
          </g>
        );
      })}
    </svg>
  );
}
