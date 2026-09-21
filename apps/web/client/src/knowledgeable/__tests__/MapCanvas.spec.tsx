import { fireEvent, render, screen } from '@testing-library/react';
import MapCanvas from '../components/MapCanvas';
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

function fixture() {
  return {
    nodes: [node(A, 'Prime Number', 0.98), node(B, 'Divisibility', 0.85), node(C, 'Factor', 0.3)],
    edges: [edge('e1', A, B, 'Dependency'), edge('e2', A, C, 'Semantic')],
  };
}

describe('MapCanvas', () => {
  it('renders node buttons with status and accessible labels', () => {
    const { nodes, edges } = fixture();
    render(<MapCanvas rootId={A} nodes={nodes} edges={edges} selectedId={A} />);
    const buttons = screen.getAllByTestId('graph-canvas-node');
    expect(buttons).toHaveLength(3);
    // Rank-major DOM order: row tops never decrease down the document.
    const tops = buttons.map((b) => Number((b as HTMLElement).style.top.replace('px', '')));
    expect([...tops].sort((a, b) => a - b)).toEqual(tops);
    const focus = buttons.find((b) => b.getAttribute('data-concept-id') === A) as HTMLElement;
    expect(focus).toHaveClass('k-node--focus');
    expect(focus).toHaveAttribute('aria-label', 'Prime Number, solid');
    expect(focus).toHaveAttribute('data-status', 'healthy');
    expect(screen.getByRole('button', { name: 'Divisibility, building' })).toHaveAttribute(
      'data-status',
      'review',
    );
  });

  it('draws arrowheads on dependency edges only', () => {
    const { nodes, edges } = fixture();
    render(<MapCanvas rootId={A} nodes={nodes} edges={edges} selectedId={A} />);
    const drawn = screen.getAllByTestId('graph-canvas-edge');
    // Selected focus A shows its semantic edge; the dependency edge carries the marker.
    const dep = drawn.find((e) => e.getAttribute('data-relation') === 'Dependency');
    expect(dep?.getAttribute('marker-end')).toBe('url(#k-arrowhead)');
    expect(dep?.getAttribute('class')).toBe('k-edge');
    const sem = drawn.find((e) => e.getAttribute('data-relation') === 'Semantic');
    expect(sem?.getAttribute('marker-end')).toBeNull();
    expect(sem?.getAttribute('class')).toContain('k-edge--related');
    expect(document.getElementById('k-arrowhead')).toBeInTheDocument();
  });

  it('hides semantic edges until their node is hovered or selected', () => {
    const { nodes, edges } = fixture();
    const { unmount } = render(<MapCanvas rootId={A} nodes={nodes} edges={edges} />);
    // No selection and no hover: only the dependency edge draws.
    expect(screen.getAllByTestId('graph-canvas-edge')).toHaveLength(1);
    unmount();

    const second = render(<MapCanvas rootId={A} nodes={nodes} edges={edges} selectedId={A} />);
    expect(screen.getAllByTestId('graph-canvas-edge')).toHaveLength(2);
    second.unmount();

    // Hover reveals the semantic edge without a selection.
    render(<MapCanvas rootId={A} nodes={nodes} edges={edges} />);
    fireEvent.mouseEnter(screen.getByRole('button', { name: 'Factor, building' }));
    expect(screen.getAllByTestId('graph-canvas-edge')).toHaveLength(2);
  });

  it('selects via click and Space', () => {
    const { nodes, edges } = fixture();
    const onSelectConcept = jest.fn();
    render(<MapCanvas rootId={A} nodes={nodes} edges={edges} onSelectConcept={onSelectConcept} />);
    const target = screen.getByRole('button', { name: 'Factor, building' });
    fireEvent.click(target);
    expect(onSelectConcept).toHaveBeenCalledWith(C);
    fireEvent.keyDown(target, { key: ' ' });
    expect(onSelectConcept).toHaveBeenCalledTimes(2);
  });

  it('renders nothing without nodes', () => {
    const { container } = render(<MapCanvas rootId={A} nodes={[]} edges={[]} />);
    expect(container).toBeEmptyDOMElement();
  });
});
