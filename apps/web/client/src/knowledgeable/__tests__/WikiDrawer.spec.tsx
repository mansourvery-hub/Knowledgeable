import { useState, type ReactElement } from 'react';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { RecoilRoot } from 'recoil';
import WikiDrawer, { splitTrySection } from '../components/WikiDrawer';
import { closeWiki, openWiki } from '../store/wikiDrawer';

const mockMarkdownRender = jest.fn();
jest.mock('~/components/Chat/Messages/Content/MarkdownBlocks', () => ({
  __esModule: true,
  default: (props: { content: string; remarkPlugins?: unknown[] }) => {
    mockMarkdownRender(props);
    return <div data-testid="wiki-markdown-stub">{props.content}</div>;
  },
}));

function renderDrawer(ui: ReactElement) {
  return render(<RecoilRoot>{ui}</RecoilRoot>);
}

const CONCEPT = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
const PREREQ = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb';

function pagePayload(over: Record<string, unknown> = {}) {
  return {
    id: 'p1',
    learner_id: 'l1',
    concept_id: CONCEPT,
    title: 'Prime Number',
    summary: 'Numbers with exactly two factors.',
    personalized_content: 'You know factors, so primes click.',
    known_prerequisites: [{ concept_id: PREREQ, name: 'Factor', learner_confidence: 0.98 }],
    related_concepts: [{ concept_id: 'c', name: 'Composite', relation_type: 'Semantic' }],
    learner_confidence_at_generation: 0.98,
    version: 1,
    is_stale: false,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
    ...over,
  };
}

function stubFetch(impl: (url: string) => { status: number; payload: unknown }) {
  (global.fetch as jest.Mock).mockImplementation((url: string) => {
    const { status, payload } = impl(url);
    return Promise.resolve({
      ok: status >= 200 && status < 300,
      status,
      json: async () => payload,
    } as Response);
  });
}

describe('splitTrySection', () => {
  it('returns the full body when there is no marker line', () => {
    expect(splitTrySection('Just body text.')).toEqual({ main: 'Just body text.', tryText: null });
  });

  it('splits a case-insensitive marker into body and try text', () => {
    const { main, tryText } = splitTrySection(
      'Body text.\n\nCHECK-for-Understanding: Why is 9 not prime?',
    );
    expect(main).toBe('Body text.');
    expect(tryText).toBe('Why is 9 not prime?');
  });
});

describe('WikiDrawer', () => {
  beforeEach(() => {
    global.fetch = jest.fn();
    closeWiki();
  });

  afterEach(() => {
    jest.resetAllMocks();
    closeWiki();
  });

  it('renders nothing without a concept', () => {
    renderDrawer(<WikiDrawer conceptId={null} />);
    expect(screen.queryByTestId('wiki-drawer')).not.toBeInTheDocument();
    expect(global.fetch).not.toHaveBeenCalled();
  });

  it('loads and renders the cached page as a labelled dialog', async () => {
    const onClose = jest.fn();
    stubFetch(() => ({ status: 200, payload: pagePayload() }));
    renderDrawer(<WikiDrawer conceptId={CONCEPT} onClose={onClose} />);

    expect(screen.getByTestId('wiki-loading')).toHaveTextContent('Writing your page');
    await waitFor(() => {
      expect(screen.getByTestId('wiki-title')).toHaveTextContent('Prime Number');
    });
    expect(screen.getByTestId('wiki-drawer')).toHaveAttribute(
      'aria-label',
      'Notebook page: Prime Number',
    );
    expect(screen.getByTestId('wiki-confidence')).toHaveTextContent('Solid, 98%');
    expect(screen.getByTestId('wiki-summary')).toHaveTextContent('exactly two factors');
    expect(screen.getByTestId('wiki-markdown-stub')).toHaveTextContent('primes click');
    expect(screen.getByTestId('wiki-prereq')).toHaveTextContent('Factor');
    expect(screen.getByTestId('wiki-related-item')).toHaveTextContent('Composite');
    expect(screen.queryByTestId('wiki-stale')).not.toBeInTheDocument();
    expect(screen.queryByTestId('wiki-try')).not.toBeInTheDocument();

    fireEvent.click(screen.getByTestId('wiki-close'));
    expect(onClose).toHaveBeenCalledTimes(1);
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(onClose).toHaveBeenCalledTimes(2);
  });

  it('flags stale pages and explains not-ready with retry', async () => {
    stubFetch(() => ({ status: 200, payload: pagePayload({ is_stale: true }) }));
    const { unmount } = renderDrawer(<WikiDrawer conceptId={CONCEPT} />);
    await waitFor(() => {
      expect(screen.getByTestId('wiki-stale')).toHaveTextContent(
        'May be outdated. It refreshes the next time you open it.',
      );
    });
    unmount();

    (global.fetch as jest.Mock).mockReset();
    stubFetch(() => ({ status: 404, payload: { code: 'wiki_not_ready', message: 'keep learning' } }));
    renderDrawer(<WikiDrawer conceptId={CONCEPT} />);
    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('Keep learning');
    });

    (global.fetch as jest.Mock).mockReset();
    stubFetch(() => ({ status: 503, payload: { code: 'service_unavailable', message: 'down' } }));
    fireEvent.click(screen.getByTestId('wiki-retry'));
    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent("Couldn't load this page");
    });
  });

  it('renders the reading typography wrapper and feeds badges to the shared pipeline', async () => {
    stubFetch(() => ({
      status: 200,
      payload: pagePayload({
        personalized_content: 'Alpha builds on Beta ideas.',
        concept_annotations: [
          { concept_id: 'b', name: 'Beta', learner_confidence: 0.85, status: 'known' },
        ],
      }),
    }));
    renderDrawer(<WikiDrawer conceptId={CONCEPT} />);
    await waitFor(() => {
      expect(screen.getByTestId('wiki-title')).toHaveTextContent('Prime Number');
    });
    // Reading typography wrapper (prose classes stay off per the redesign).
    const content = screen.getByTestId('wiki-content');
    expect(content).toHaveClass('k-read');
    expect(content).not.toHaveClass('prose');
    // Same markdown pipeline as chat, fed with the page annotations.
    const calls = mockMarkdownRender.mock.calls;
    const props = calls[calls.length - 1][0] as {
      remarkPlugins?: unknown[];
    };
    const highlight = (props.remarkPlugins ?? []).find(
      (entry): entry is [unknown, { annotations: { name: string }[] }] =>
        Array.isArray(entry) &&
        typeof entry[1] === 'object' &&
        entry[1] !== null &&
        'annotations' in entry[1],
    );
    expect(highlight?.[1].annotations).toMatchObject([{ name: 'Beta' }]);
  });

  it('moves the try-this section out of the body', async () => {
    stubFetch(() => ({
      status: 200,
      payload: pagePayload({
        personalized_content: 'Body text.\n\nCheck-for-understanding: Why is 9 not prime?',
      }),
    }));
    renderDrawer(<WikiDrawer conceptId={CONCEPT} />);
    await waitFor(() => {
      expect(screen.getByTestId('wiki-try')).toBeInTheDocument();
    });
    expect(screen.getByTestId('wiki-try')).toHaveTextContent('Try this');
    expect(screen.getByTestId('wiki-try')).toHaveTextContent('Why is 9 not prime?');
    const body = screen.getByTestId('wiki-content');
    expect(body).toHaveTextContent('Body text.');
    expect(body).not.toHaveTextContent('Why is 9');
  });

  it('navigates via chips with a working Back button', async () => {
    stubFetch((url) => {
      if (url.includes(PREREQ)) {
        return { status: 200, payload: pagePayload({ concept_id: PREREQ, title: 'Factor' }) };
      }
      return { status: 200, payload: pagePayload() };
    });
    openWiki(CONCEPT);
    renderDrawer(<WikiDrawer />);
    await waitFor(() => {
      expect(screen.getByTestId('wiki-title')).toHaveTextContent('Prime Number');
    });
    expect(screen.queryByRole('button', { name: 'Back' })).not.toBeInTheDocument();

    fireEvent.click(screen.getByTestId('wiki-prereq'));
    await waitFor(() => {
      expect(screen.getByTestId('wiki-title')).toHaveTextContent('Factor');
    });
    // Navigating to a new page moves focus back to the close button.
    expect(screen.getByTestId('wiki-close')).toHaveFocus();
    fireEvent.click(screen.getByRole('button', { name: 'Back' }));
    await waitFor(() => {
      expect(screen.getByTestId('wiki-title')).toHaveTextContent('Prime Number');
    });
  });

  it('focuses the close button on open and restores the opener on close', async () => {
    stubFetch(() => ({ status: 200, payload: pagePayload() }));
    function Controlled() {
      const [id, setId] = useState<string | null>(null);
      return (
        <>
          <button type="button" data-testid="opener" onClick={() => setId(CONCEPT)}>
            open
          </button>
          <WikiDrawer conceptId={id} onClose={() => setId(null)} />
        </>
      );
    }
    renderDrawer(<Controlled />);
    (screen.getByTestId('opener') as HTMLElement).focus();
    fireEvent.click(screen.getByTestId('opener'));
    await waitFor(() => {
      expect(screen.getByTestId('wiki-title')).toBeInTheDocument();
    });
    expect(screen.getByTestId('wiki-close')).toHaveFocus();

    fireEvent.click(screen.getByTestId('wiki-close'));
    expect(screen.getByTestId('opener')).toHaveFocus();
  });
});
