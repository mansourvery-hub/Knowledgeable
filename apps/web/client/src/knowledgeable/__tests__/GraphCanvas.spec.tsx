import { fireEvent, render, screen } from '@testing-library/react';
import GraphCanvas, { shortLabel } from '../components/GraphCanvas';
import type { ConceptRelation, NeighborhoodNode } from '../graphTypes';

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

const NODES = [
  node(A, 'Alpha Prime Concept', 0.98),
  node(B, 'Beta', 0.3),
  node(C, 'Gamma', null),
];
const EDGES: ConceptRelation[] = [
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
];

describe('GraphCanvas', () => {
  it('renders positioned nodes and typed edges', () => {
    render(<GraphCanvas rootId={A} nodes={NODES} edges={EDGES} />);
    const canvas = screen.getByTestId('graph-canvas');
    expect(canvas).toHaveAttribute('viewBox');

    const rendered = screen.getAllByTestId('graph-canvas-node');
    expect(rendered).toHaveLength(3);
    expect(rendered[0]).toHaveAttribute('data-concept-id', A);
    expect(rendered[0]).toHaveAttribute('data-status', 'healthy');
    expect(rendered[0].getAttribute('transform')).toContain('translate(');

    const edges = screen.getAllByTestId('graph-canvas-edge');
    expect(edges).toHaveLength(2);
    const dep = edges.find((e) => e.getAttribute('data-relation') === 'Dependency');
    expect(dep?.getAttribute('marker-end')).toContain('graph-canvas-arrow');
    expect(dep?.getAttribute('stroke-dasharray')).toBeNull();
    const sem = edges.find((e) => e.getAttribute('data-relation') === 'Semantic');
    expect(sem?.getAttribute('stroke-dasharray')).toBe('5 4');
    expect(sem?.getAttribute('marker-end')).toBeNull();
  });

  it('truncates long labels but keeps full names for hover', () => {
    expect(shortLabel('Alpha Prime Concept')).toBe('Alpha Prime Con…');
    expect(shortLabel('Beta')).toBe('Beta');
    render(<GraphCanvas rootId={A} nodes={NODES} edges={EDGES} />);
    expect(screen.getByText('Alpha Prime Con…')).toBeInTheDocument();
  });

  it('notifies on click and keyboard activation', () => {
    const onSelectConcept = jest.fn();
    render(<GraphCanvas rootId={A} nodes={NODES} edges={EDGES} onSelectConcept={onSelectConcept} />);
    const beta = screen
      .getAllByTestId('graph-canvas-node')
      .find((n) => n.getAttribute('data-concept-id') === B);
    expect(beta).toHaveAttribute('role', 'button');
    fireEvent.click(beta as Element);
    expect(onSelectConcept).toHaveBeenCalledWith(B);
    fireEvent.keyDown(beta as Element, { key: 'Enter' });
    expect(onSelectConcept).toHaveBeenCalledTimes(2);
  });

  it('renders nothing without nodes', () => {
    const { container } = render(<GraphCanvas rootId={A} nodes={[]} edges={[]} />);
    expect(container).toBeEmptyDOMElement();
  });
});
