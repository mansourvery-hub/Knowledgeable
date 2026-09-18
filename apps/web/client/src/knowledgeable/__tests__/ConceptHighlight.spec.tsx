import { fireEvent, render, screen } from '@testing-library/react';
import ConceptHighlight from '../components/ConceptHighlight';
import { closeWiki, getOpenWikiConceptId } from '../store/wikiDrawer';

describe('ConceptHighlight', () => {
  it('renders badge styles per status with an accessible label', () => {
    const { rerender } = render(
      <ConceptHighlight name="Prime Number" status="known" confidence={0.98}>
        Prime Number
      </ConceptHighlight>,
    );
    const badge = screen.getByTestId('concept-highlight');
    expect(badge).toHaveAttribute('data-status', 'known');
    expect(badge).toHaveClass('border-dotted');
    expect(badge).toHaveAttribute(
      'aria-label',
      'Prime Number, Known concept, confidence 98%',
    );
    expect(screen.getByTestId('concept-highlight-confidence')).toHaveTextContent('98%');

    rerender(
      <ConceptHighlight name="Factor" status="weak" confidence={0.3}>
        Factor
      </ConceptHighlight>,
    );
    expect(screen.getByTestId('concept-highlight')).toHaveClass('bg-amber-500/15');
    expect(screen.getByTestId('concept-highlight-tooltip')).toHaveTextContent(
      'Needs review',
    );

    rerender(
      <ConceptHighlight name="Unseen Thing" status="new" confidence={null}>
        Unseen Thing
      </ConceptHighlight>,
    );
    expect(screen.getByTestId('concept-highlight')).toHaveClass('bg-blue-500/15');
    expect(screen.getByTestId('concept-highlight-confidence')).toHaveTextContent('unseen');
  });

  it('degrades to plain text on invalid props', () => {
    render(
      // @ts-expect-error invalid status must not crash render
      <ConceptHighlight name="X" status="bogus">
        X
      </ConceptHighlight>,
    );
    expect(screen.queryByTestId('concept-highlight')).not.toBeInTheDocument();
    expect(screen.getByText('X')).toBeInTheDocument();
  });

  it('opens the wiki drawer on click when a concept id is present', () => {
    closeWiki();
    const id = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
    render(
      <ConceptHighlight conceptId={id} name="Prime Number" status="known" confidence={0.98}>
        Prime Number
      </ConceptHighlight>,
    );
    const badge = screen.getByTestId('concept-highlight');
    expect(badge).toHaveAttribute('role', 'button');
    fireEvent.click(badge);
    expect(getOpenWikiConceptId()).toBe(id);
    closeWiki();
  });
});
