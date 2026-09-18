import { useMemo, useState } from 'react';
import GraphCanvas from './GraphCanvas';
import type { Neighborhood } from '../graphTypes';
import {
  confidenceStatus,
  countNeighborhood,
  edgeLabel,
  filterReviewOnly,
  formatConfidence,
  sortByConfidenceAscending,
} from '../graphUtils';

export interface GraphExplorerProps {
  /** Bounded neighborhood from `fetchNeighborhood`; `null` = not loaded yet. */
  neighborhood: Neighborhood | null;
  loading?: boolean;
  /** Rendered in a `role="alert"` region with an optional retry action. */
  error?: string | null;
  /** Called when a node is activated. Chat remains the primary surface. */
  onSelectConcept?: (conceptId: string) => void;
  onRetry?: () => void;
  /** Controlled review-only filter; omit for uncontrolled (internal state). */
  showReviewOnly?: boolean;
  onToggleReviewOnly?: (value: boolean) => void;
  /** Queried root concept: canvas center + review-filter anchor. */
  rootConceptId?: string;
}

/**
 * T9 neighborhood view (M9): read-only inspection panel consuming
 * `GET /api/graph/neighborhood`.
 *
 * - Distinguishes semantic (dashed) vs dependency (solid directed) edges.
 * - Surfaces learner confidence + needs-review filtering.
 * - Provider-free: no Recoil/react-query imports, safe in Jest + any route.
 */
export default function GraphExplorer({
  neighborhood,
  loading = false,
  error = null,
  onSelectConcept,
  onRetry,
  showReviewOnly,
  onToggleReviewOnly,
  rootConceptId,
}: GraphExplorerProps) {
  const [internalReviewOnly, setInternalReviewOnly] = useState(false);
  const reviewOnly = showReviewOnly ?? internalReviewOnly;
  const setReviewOnly = (value: boolean) => {
    if (onToggleReviewOnly) {
      onToggleReviewOnly(value);
    } else {
      setInternalReviewOnly(value);
    }
  };

  const nodes = useMemo(
    () => sortByConfidenceAscending(neighborhood?.nodes ?? []),
    [neighborhood],
  );
  const edges = useMemo(() => neighborhood?.edges ?? [], [neighborhood]);
  const visibleNodes = useMemo(
    () => (reviewOnly ? filterReviewOnly(nodes) : nodes),
    [nodes, reviewOnly],
  );
  const rootId = rootConceptId ?? nodes[0]?.concept.id ?? '';
  // The canvas honors the review filter but always keeps the queried root
  // as its orientation anchor (it may itself be healthy).
  const canvasNodes = useMemo(() => {
    if (!reviewOnly) {
      return nodes;
    }
    const ids = new Set(visibleNodes.map((n) => n.concept.id));
    ids.add(rootId);
    return nodes.filter((n) => ids.has(n.concept.id));
  }, [nodes, visibleNodes, reviewOnly, rootId]);
  const counts = useMemo(() => countNeighborhood(nodes, edges), [nodes, edges]);
  const names = useMemo(() => {
    const map = new Map<string, string>();
    for (const node of nodes) {
      map.set(node.concept.id, node.concept.canonical_name);
    }
    return map;
  }, [nodes]);

  if (loading) {
    return (
      <section aria-label="Knowledge graph neighborhood" data-testid="graph-explorer">
        <h2>Graph Explorer</h2>
        <div role="status" data-testid="graph-loading">
          Loading graph neighborhood…
        </div>
      </section>
    );
  }

  if (error) {
    return (
      <section aria-label="Knowledge graph neighborhood" data-testid="graph-explorer">
        <h2>Graph Explorer</h2>
        <div role="alert" data-testid="graph-error">
          {error}
        </div>
        {onRetry && (
          <button type="button" data-testid="graph-retry" onClick={onRetry}>
            Retry
          </button>
        )}
      </section>
    );
  }

  if (!neighborhood || nodes.length === 0) {
    return (
      <section aria-label="Knowledge graph neighborhood" data-testid="graph-explorer">
        <h2>Graph Explorer</h2>
        <p data-testid="graph-empty">No graph data yet.</p>
      </section>
    );
  }

  return (
    <section aria-label="Knowledge graph neighborhood" data-testid="graph-explorer">
      <h2>Graph Explorer</h2>
      <p data-testid="graph-counts">
        {counts.concepts} concepts · {counts.links} links · {counts.needReview} need review
      </p>

      <ul data-testid="graph-legend" aria-label="Edge legend">
        <li data-testid="graph-legend-dependency">
          <span aria-hidden="true">—▶</span> depends on (dependency)
        </li>
        <li data-testid="graph-legend-semantic">
          <span aria-hidden="true">┄</span> related (semantic)
        </li>
      </ul>

      <GraphCanvas
        rootId={rootId}
        nodes={canvasNodes}
        edges={edges}
        onSelectConcept={onSelectConcept}
      />

      <label>
        <input
          type="checkbox"
          data-testid="graph-review-toggle"
          checked={reviewOnly}
          onChange={(event) => setReviewOnly(event.target.checked)}
        />
        Show needs-review only
      </label>

      {visibleNodes.length === 0 ? (
        <p data-testid="graph-empty">No concepts need review.</p>
      ) : (
        <ul data-testid="graph-nodes" aria-label="Concepts">
          {visibleNodes.map((node) => {
            const status = confidenceStatus(node.learner_confidence);
            const name = node.concept.canonical_name;
            return (
              <li
                key={node.concept.id}
                data-testid="graph-node"
                data-concept-id={node.concept.id}
                data-status={status}
              >
                {onSelectConcept ? (
                  <button
                    type="button"
                    data-testid={`graph-node-select-${node.concept.id}`}
                    onClick={() => onSelectConcept(node.concept.id)}
                  >
                    {name}
                  </button>
                ) : (
                  <strong>{name}</strong>
                )}{' '}
                <span data-testid="graph-node-confidence">
                  {formatConfidence(node.learner_confidence)}
                </span>{' '}
                <span data-testid="graph-node-status">{status}</span>
                <div
                  aria-hidden="true"
                  data-testid="graph-node-bar"
                  style={{
                    width: `${Math.round((node.learner_confidence ?? 0) * 100)}%`,
                  }}
                />
              </li>
            );
          })}
        </ul>
      )}

      {edges.length > 0 && (
        <ul data-testid="graph-edges" aria-label="Relations">
          {edges.map((edge) => (
            <li
              key={edge.id}
              data-testid="graph-edge"
              data-relation={edge.relation_type}
            >
              {names.get(edge.from_concept_id) ?? edge.from_concept_id}{' '}
              {edge.relation_type === 'Dependency' ? (
                <span aria-hidden="true">—▶</span>
              ) : (
                <span aria-hidden="true">┄</span>
              )}{' '}
              {edgeLabel(edge.relation_type)}{' '}
              {names.get(edge.to_concept_id) ?? edge.to_concept_id}
            </li>
          ))}
        </ul>
      )}

      <p data-testid="graph-note">Chat remains primary; this panel is read-only inspection.</p>
    </section>
  );
}
