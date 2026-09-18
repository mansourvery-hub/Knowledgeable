/**
 * Pipeline integration for M7 rendering (T11d).
 *
 * Renders through the real shared config (`getRemarkPlugins` /
 * `getMarkdownComponents`) to prove the `concept-highlight` hName maps to
 * `<ConceptHighlight>` and that code/math survive the full pipeline.
 */
import { render, screen } from '@testing-library/react';
import type { ElementType } from 'react';
import ReactMarkdown from 'react-markdown';
import {
  getMarkdownComponents,
  getRehypePlugins,
  getRemarkPlugins,
} from '../../components/Chat/Messages/Content/markdownConfig';
import ConceptHighlight from '../components/ConceptHighlight';
import type { ConceptAnnotation } from '../types';

const ANNOTATIONS: ConceptAnnotation[] = [
  {
    concept_id: 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
    name: 'Prime Number',
    learner_confidence: 0.98,
    status: 'known',
  },
  {
    concept_id: 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb',
    name: 'Factor',
    learner_confidence: 0.3,
    status: 'weak',
  },
];

function renderMarkdown(markdown: string) {
  // Minimal components: the shared config's `code` renderer needs its
  // provider tree, so render prose/badges here and assert the registration
  // mapping separately below. (Record annotation mirrors
  // `getMarkdownComponents` and dodges the literal excess-key check.)
  const components: Record<string, ElementType> = {
    'concept-highlight': ConceptHighlight,
  };
  return render(
    <ReactMarkdown
      remarkPlugins={getRemarkPlugins(true, ANNOTATIONS)}
      rehypePlugins={getRehypePlugins()}
      components={components}
    >
      {markdown}
    </ReactMarkdown>,
  );
}

describe('concept highlight pipeline', () => {
  it('registers concept-highlight in the shared components map', () => {
    expect(getMarkdownComponents()['concept-highlight']).toBe(ConceptHighlight);
  });
  it('badges known prose mentions while sparing code, math, and weak mentions', () => {
    const { container } = renderMarkdown(
      'A Prime Number uses each Factor once.\n\n`Factor` in code and $Prime Number$ in math.\n\n```\nPrime Number\n```',
    );
    // T25: only the known mention badges; the weak one stays plain prose.
    const badges = screen.getAllByTestId('concept-highlight');
    expect(badges).toHaveLength(1);
    expect(badges[0]).toHaveAttribute('data-status', 'known');
    expect(screen.getByText(/uses each/)).toBeInTheDocument();
    // Code and math render their literal text with no badge inside.
    expect(container.querySelector('code')).not.toBeNull();
    const codeText = container.querySelector('code')?.textContent ?? '';
    expect(codeText).toContain('Factor');
    expect(
      container.querySelector('[data-testid="concept-highlight"] code'),
    ).toBeNull();
  });

  it('renders nothing extra while streaming (no annotations yet)', () => {
    const components: Record<string, ElementType> = {
      'concept-highlight': ConceptHighlight,
    };
    render(
      <ReactMarkdown
        remarkPlugins={getRemarkPlugins(true, [])}
        rehypePlugins={getRehypePlugins()}
        components={components}
      >
        {'A Prime Number'}
      </ReactMarkdown>,
    );
    expect(screen.queryByTestId('concept-highlight')).not.toBeInTheDocument();
    expect(screen.getByText(/Prime Number/)).toBeInTheDocument();
  });
});
