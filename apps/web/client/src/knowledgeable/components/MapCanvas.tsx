/**
 * Map canvas (Phase 2, SPEC 6.4): replaces `GraphCanvas.tsx`.
 *
 * Scroll container `.k-canvas` > stage sized from `layoutLayered` >
 * `svg.k-edges` underneath + one `<button class="k-node">` per concept.
 * Dependency edges always draw with the `k-arrowhead` marker; semantic
 * edges draw only for the hovered or selected node.
 */
import { useEffect, useMemo, useRef, useState } from 'react';
import type { KeyboardEvent } from 'react';
import type { ConceptRelation, NeighborhoodNode } from '../graphTypes';
import { confidenceStatus } from '../graphUtils';
import { layoutLayered } from '../mapLayout';
import ConfidenceRing from './ui/ConfidenceRing';

export interface MapCanvasProps {
  rootId: string;
  nodes: NeighborhoodNode[];
  edges: ConceptRelation[];
  selectedId?: string;
  onSelectConcept?: (conceptId: string) => void;
}

/** Lowercase words for node `aria-label`s ("Name, solid"); visible copy uses `confidenceWords`. */
const ARIA_WORDS = { healthy: 'solid', review: 'building', unseen: 'not met yet' } as const;

export default function MapCanvas({ rootId, nodes, edges, selectedId, onSelectConcept }: MapCanvasProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [minSize, setMinSize] = useState({ width: 0, height: 0 });
  const [hoveredId, setHoveredId] = useState<string | null>(null);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) {
      return;
    }
    const measure = () => {
      setMinSize((prev) =>
        prev.width === el.clientWidth && prev.height === el.clientHeight
          ? prev
          : { width: el.clientWidth, height: el.clientHeight },
      );
    };
    measure();
    // jsdom ships a ResizeObserver stub without observe(); guard the method,
    // not just the global, so tests and old browsers skip live measuring.
    try {
      const observer = new ResizeObserver(measure);
      if (typeof observer.observe !== 'function') {
        return;
      }
      observer.observe(el);
      return () => observer.disconnect();
    } catch {
      return;
    }
  }, []);

  const confidenceById = useMemo(() => new Map(nodes.map((n) => [n.concept.id, n.learner_confidence])), [nodes]);
  const layout = useMemo(
    () => layoutLayered(rootId, nodes, edges, minSize),
    [rootId, nodes, edges, minSize],
  );

  // Initial scroll centres the focus node (direct assignment: jsdom-safe).
  useEffect(() => {
    const el = containerRef.current;
    const focus = layout.nodes.find((n) => n.focus) ?? layout.nodes.find((n) => n.id === selectedId);
    if (!el || !focus) {
      return;
    }
    el.scrollLeft = Math.max(0, focus.x - el.clientWidth / 2);
    el.scrollTop = Math.max(0, focus.y - el.clientHeight / 2);
  }, [layout, selectedId]);

  if (layout.nodes.length === 0) {
    return null;
  }

  const activeId = hoveredId ?? selectedId ?? null;
  const dependencyEdges = layout.edges.filter((e) => e.relation === 'Dependency');
  const semanticEdges = layout.edges.filter(
    (e) => e.relation === 'Semantic' && (e.fromId === activeId || e.toId === activeId),
  );

  const select = (id: string) => onSelectConcept?.(id);
  const handleKeyDown = (event: KeyboardEvent<HTMLButtonElement>, id: string) => {
    // Enter activates natively via onClick; Space needs a manual handler
    // (preventDefault suppresses the native keyup click, so no double-fire).
    if (event.key === ' ') {
      event.preventDefault();
      select(id);
    }
  };

  return (
    <div ref={containerRef} className="k-canvas" data-testid="graph-canvas">
      <div className="k-canvas__stage" style={{ width: layout.width, height: layout.height }}>
        <svg className="k-edges" viewBox={`0 0 ${layout.width} ${layout.height}`} aria-hidden="true">
          <defs>
            <marker id="k-arrowhead" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="8" markerHeight="8" orient="auto">
              <path className="k-arrow" d="M1 1.5L8.5 5L1 8.5" />
            </marker>
          </defs>
          {dependencyEdges.map((edge) => (
            <path
              key={edge.id}
              className="k-edge"
              d={edge.d}
              markerEnd="url(#k-arrowhead)"
              data-testid="graph-canvas-edge"
              data-relation="Dependency"
            />
          ))}
          {semanticEdges.map((edge) => (
            <path
              key={edge.id}
              className="k-edge k-edge--related"
              d={edge.d}
              data-testid="graph-canvas-edge"
              data-relation="Semantic"
            />
          ))}
        </svg>
        {layout.nodes.map((pill) => {
          const status = confidenceStatus(confidenceById.get(pill.id) ?? null);
          return (
            <button
              key={pill.id}
              type="button"
              className={pill.focus ? 'k-node k-node--focus' : 'k-node'}
              style={{ left: pill.x, top: pill.y }}
              aria-label={`${pill.name}, ${ARIA_WORDS[status]}`}
              title={pill.name}
              data-testid="graph-canvas-node"
              data-concept-id={pill.id}
              data-status={status}
              onClick={() => select(pill.id)}
              onKeyDown={(event) => handleKeyDown(event, pill.id)}
              onMouseEnter={() => setHoveredId(pill.id)}
              onMouseLeave={() => setHoveredId(null)}
              onFocus={() => setHoveredId(pill.id)}
              onBlur={() => setHoveredId(null)}
            >
              <ConfidenceRing value={confidenceById.get(pill.id) ?? null} size={pill.focus ? 22 : 20} />
              <span className="k-node__label">{pill.label}</span>
            </button>
          );
        })}
      </div>
    </div>
  );
}
