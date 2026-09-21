import type { ReactElement } from 'react';
import { fireEvent, render, screen } from '@testing-library/react';
import { RecoilRoot } from 'recoil';
import ConceptHighlight from '../components/ConceptHighlight';
import { closeWiki, getOpenWikiConceptId } from '../store/wikiDrawer';
import store from '~/store';

/* F7/F8 product call: confidence percentages render only with the debug
 * toggle on, so every render carries an explicit atom value. */
function renderBadge(ui: ReactElement, debug = false) {
  return render(
    <RecoilRoot initializeState={({ set }) => set(store.showConfidenceDebug, debug)}>
      {ui}
    </RecoilRoot>,
  );
}

describe('ConceptHighlight', () => {
  it('renders badge styles per status without percentages by default', () => {
    const { rerender } = renderBadge(
      <ConceptHighlight name="Prime Number" status="known" confidence={0.98}>
        Prime Number
      </ConceptHighlight>,
    );
    const badge = screen.getByTestId('concept-highlight');
    expect(badge).toHaveAttribute('data-status', 'known');
    expect(badge).toHaveClass('k-concept', 'k-concept--known');
    expect(badge).toHaveAttribute('aria-label', 'Prime Number, Known concept');
    expect(screen.queryByTestId('concept-highlight-confidence')).not.toBeInTheDocument();

    rerender(
      <RecoilRoot initializeState={({ set }) => set(store.showConfidenceDebug, false)}>
        <ConceptHighlight name="Factor" status="weak" confidence={0.3}>
          Factor
        </ConceptHighlight>
      </RecoilRoot>,
    );
    expect(screen.getByTestId('concept-highlight')).toHaveClass('k-concept', 'k-concept--weak');
    expect(screen.getByTestId('concept-highlight-tooltip')).toHaveTextContent('Needs review');
    expect(screen.getByTestId('concept-highlight-tooltip')).toHaveClass('k-tip');
  });

  it('shows percentages with the debug toggle on', () => {
    renderBadge(
      <ConceptHighlight name="Prime Number" status="known" confidence={0.98}>
        Prime Number
      </ConceptHighlight>,
      true,
    );
    expect(screen.getByTestId('concept-highlight')).toHaveAttribute(
      'aria-label',
      'Prime Number, Known concept, confidence 98%',
    );
    expect(screen.getByTestId('concept-highlight-confidence')).toHaveTextContent('98%');
  });

  it('degrades unknown confidence without crashing', () => {
    renderBadge(
      <ConceptHighlight name="Unseen Thing" status="new" confidence={null} />,
      true,
    );
    expect(screen.getByTestId('concept-highlight-confidence')).toHaveTextContent('unseen');
  });

  it('degrades to plain text on invalid props', () => {
    renderBadge(
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
    renderBadge(
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
