import { fireEvent, render, screen, within } from '@testing-library/react';
import GraphExplorer from '../components/GraphExplorer';
import { closeWiki, getOpenWikiConceptId } from '../store/wikiDrawer';
import type { Neighborhood } from '../graphTypes';

const A = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
const B = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb';
const C = 'cccccccc-cccc-cccc-cccc-cccccccccccc';

function fixture(): Neighborhood {
  return {
    nodes: [
      {
        concept: {
          id: A,
          canonical_name: 'Alpha',
          canonical_statement: 'Alpha statement.',
          learner_statement: null,
          world_confidence: 1.0,
          status: 'Active',
          created_at: '2026-01-01T00:00:00Z',
          updated_at: '2026-01-01T00:00:00Z',
        },
        learner_confidence: 0.98,
        is_healthy: true,
        is_review_eligible: false,
      },
      {
        concept: {
          id: B,
          canonical_name: 'Beta',
          canonical_statement: 'Beta statement.',
          learner_statement: null,
          world_confidence: 1.0,
          status: 'Active',
          created_at: '2026-01-01T00:00:00Z',
          updated_at: '2026-01-01T00:00:00Z',
        },
        learner_confidence: 0.3,
        is_healthy: false,
        is_review_eligible: true,
      },
      {
        concept: {
          id: C,
          canonical_name: 'Gamma',
          canonical_statement: 'Gamma statement.',
          learner_statement: null,
          world_confidence: 1.0,
          status: 'Active',
          created_at: '2026-01-01T00:00:00Z',
          updated_at: '2026-01-01T00:00:00Z',
        },
        learner_confidence: null,
        is_healthy: null,
        is_review_eligible: null,
      },
    ],
    edges: [
      {
        id: 'e1',
        from_concept_id: A,
        to_concept_id: B,
        relation_type: 'Dependency',
        created_at: '2026-01-01T00:00:00Z',
        updated_at: '2026-01-01T00:00:00Z',
      },
      {
        id: 'e2',
        from_concept_id: A,
        to_concept_id: C,
        relation_type: 'Semantic',
        created_at: '2026-01-01T00:00:00Z',
        updated_at: '2026-01-01T00:00:00Z',
      },
    ],
  };
}

describe('GraphExplorer', () => {
  it('renders counts, confidence, and the edge-type legend', () => {
    render(<GraphExplorer neighborhood={fixture()} />);
    expect(screen.getByTestId('graph-counts')).toHaveTextContent(
      '3 concepts · 2 links · 1 need review',
    );
    expect(screen.getByTestId('graph-legend-dependency')).toHaveTextContent('dependency');
    expect(screen.getByTestId('graph-legend-semantic')).toHaveTextContent('semantic');

    const nodes = screen.getAllByTestId('graph-node');
    expect(nodes).toHaveLength(3);
    // Weakest known confidence sorts first.
    expect(nodes[0]).toHaveAttribute('data-concept-id', B);
    expect(within(nodes[0]).getByTestId('graph-node-confidence')).toHaveTextContent('30%');
    expect(within(nodes[0]).getByTestId('graph-node-status')).toHaveTextContent('review');

    const edges = screen.getAllByTestId('graph-edge');
    expect(edges).toHaveLength(2);
    expect(edges[0]).toHaveAttribute('data-relation', 'Dependency');
    expect(edges[0]).toHaveTextContent('depends on');
    expect(edges[1]).toHaveAttribute('data-relation', 'Semantic');
    expect(edges[1]).toHaveTextContent('related');
  });

  it('filters to the review/weak view when toggled', () => {
    render(<GraphExplorer neighborhood={fixture()} />);
    fireEvent.click(screen.getByTestId('graph-review-toggle'));
    const nodes = screen.getAllByTestId('graph-node');
    expect(nodes).toHaveLength(1);
    expect(nodes[0]).toHaveAttribute('data-concept-id', B);
  });

  it('notifies the parent when a node is selected', () => {
    const onSelectConcept = jest.fn();
    render(<GraphExplorer neighborhood={fixture()} onSelectConcept={onSelectConcept} />);
    fireEvent.click(screen.getByTestId(`graph-node-select-${B}`));
    expect(onSelectConcept).toHaveBeenCalledWith(B);
  });

  it('opens the wiki drawer when a node wiki button is activated', () => {
    closeWiki();
    render(<GraphExplorer neighborhood={fixture()} onSelectConcept={jest.fn()} />);
    fireEvent.click(screen.getByTestId(`graph-node-wiki-${B}`));
    expect(getOpenWikiConceptId()).toBe(B);
    closeWiki();
  });

  it('renders loading, error+retry, and empty states accessibly', () => {
    const { unmount } = render(<GraphExplorer neighborhood={null} loading={true} />);
    expect(screen.getByTestId('graph-loading')).toHaveTextContent('Loading');
    unmount();

    const onRetry = jest.fn();
    render(<GraphExplorer neighborhood={null} error="graph service unavailable" onRetry={onRetry} />);
    expect(screen.getByRole('alert')).toHaveTextContent('graph service unavailable');
    fireEvent.click(screen.getByTestId('graph-retry'));
    expect(onRetry).toHaveBeenCalled();
  });

  it('renders the empty state when no neighborhood is loaded', () => {
    render(<GraphExplorer neighborhood={null} />);
    expect(screen.getByTestId('graph-empty')).toHaveTextContent('No graph data yet.');
  });

  it('renders the canvas map and keeps the root anchored under the review filter', () => {
    const onSelectConcept = jest.fn();
    render(<GraphExplorer neighborhood={fixture()} rootConceptId={A} onSelectConcept={onSelectConcept} />);
    // Full map: all 3 nodes + both edges.
    expect(screen.getAllByTestId('graph-canvas-node')).toHaveLength(3);
    expect(screen.getAllByTestId('graph-canvas-edge')).toHaveLength(2);

    // Review-only: list narrows to Beta, canvas keeps Beta + root Alpha.
    fireEvent.click(screen.getByTestId('graph-review-toggle'));
    expect(screen.getAllByTestId('graph-node')).toHaveLength(1);
    const canvasNodes = screen.getAllByTestId('graph-canvas-node');
    expect(canvasNodes).toHaveLength(2);
    const ids = canvasNodes.map((n) => n.getAttribute('data-concept-id'));
    expect(ids).toContain(A);
    expect(ids).toContain(B);

    // Canvas selection drills like the list.
    fireEvent.click(canvasNodes.find((n) => n.getAttribute('data-concept-id') === B) as Element);
    expect(onSelectConcept).toHaveBeenCalledWith(B);
  });
});
