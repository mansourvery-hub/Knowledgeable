import type { ReactElement } from 'react';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { RecoilRoot } from 'recoil';
import WikiDrawer from '../components/WikiDrawer';
import store from '~/store';

const mockMarkdownRender = jest.fn();
jest.mock('~/components/Chat/Messages/Content/MarkdownBlocks', () => ({
  __esModule: true,
  default: (props: { content: string; remarkPlugins?: unknown[] }) => {
    mockMarkdownRender(props);
    return <div data-testid="wiki-markdown-stub">{props.content}</div>;
  },
}));

/* F7 render parity: the drawer reads persisted atoms now, so every render
 * carries an explicit debug value (percentages default off). */
function renderDrawer(ui: ReactElement, debug = false) {
  return render(
    <RecoilRoot initializeState={({ set }) => set(store.showConfidenceDebug, debug)}>
      {ui}
    </RecoilRoot>,
  );
}

const CONCEPT = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';

function pagePayload(over: Record<string, unknown> = {}) {
  return {
    id: 'p1',
    learner_id: 'l1',
    concept_id: CONCEPT,
    title: 'Prime Number',
    summary: 'Numbers with exactly two factors.',
    personalized_content: 'You know factors, so primes click.',
    known_prerequisites: [
      { concept_id: 'b', name: 'Factor', learner_confidence: 0.98 },
    ],
    related_concepts: [{ concept_id: 'c', name: 'Composite', relation_type: 'Semantic' }],
    learner_confidence_at_generation: 0.98,
    version: 1,
    is_stale: false,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
    ...over,
  };
}

function stubFetch(status: number, payload: unknown) {
  (global.fetch as jest.Mock).mockResolvedValue({
    ok: status >= 200 && status < 300,
    status,
    json: async () => payload,
  } as Response);
}

describe('WikiDrawer', () => {
  beforeEach(() => {
    global.fetch = jest.fn();
  });

  afterEach(() => {
    jest.resetAllMocks();
  });

  it('renders nothing without a concept', () => {
    renderDrawer(<WikiDrawer conceptId={null} />);
    expect(screen.queryByTestId('wiki-drawer')).not.toBeInTheDocument();
    expect(global.fetch).not.toHaveBeenCalled();
  });

  it('loads and renders the cached page with anchors and gauge', async () => {
    const onClose = jest.fn();
    stubFetch(200, pagePayload());
    renderDrawer(<WikiDrawer conceptId={CONCEPT} onClose={onClose} />, true);

    expect(screen.getByTestId('wiki-loading')).toBeInTheDocument();
    await waitFor(() => {
      expect(screen.getByTestId('wiki-title')).toHaveTextContent('Prime Number');
    });
    expect(screen.getByTestId('wiki-confidence')).toHaveTextContent('98%');
    expect(screen.getByTestId('wiki-summary')).toHaveTextContent('exactly two factors');
    expect(screen.getByTestId('wiki-markdown-stub')).toHaveTextContent('primes click');
    expect(screen.getByTestId('wiki-prereq')).toHaveTextContent('Factor · 98%');
    expect(screen.getByTestId('wiki-related-item')).toHaveTextContent('Composite');
    expect(screen.queryByTestId('wiki-stale')).not.toBeInTheDocument();

    fireEvent.click(screen.getByTestId('wiki-close'));
    expect(onClose).toHaveBeenCalledTimes(1);
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(onClose).toHaveBeenCalledTimes(2);
  });

  it('flags stale pages and explains not-ready with retry', async () => {
    stubFetch(200, pagePayload({ is_stale: true }));
    const { unmount } = renderDrawer(<WikiDrawer conceptId={CONCEPT} />);
    await waitFor(() => {
      expect(screen.getByTestId('wiki-stale')).toBeInTheDocument();
    });
    unmount();

    (global.fetch as jest.Mock).mockReset();
    stubFetch(404, { code: 'wiki_not_ready', message: 'keep learning' });
    renderDrawer(<WikiDrawer conceptId={CONCEPT} />);
    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('keep learning');
    });

    (global.fetch as jest.Mock).mockReset();
    stubFetch(503, { code: 'service_unavailable', message: 'down' });
    fireEvent.click(screen.getByTestId('wiki-retry'));
    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent("Couldn't load");
    });
  });

  it('hides percentages by default and feeds badges to the shared pipeline', async () => {
    stubFetch(
      200,
      pagePayload({
        personalized_content: 'Alpha builds on Beta ideas.',
        concept_annotations: [
          { concept_id: 'b', name: 'Beta', learner_confidence: 0.85, status: 'known' },
        ],
      }),
    );
    renderDrawer(<WikiDrawer conceptId={CONCEPT} />);
    await waitFor(() => {
      expect(screen.getByTestId('wiki-title')).toHaveTextContent('Prime Number');
    });
    // Debug toggle off: gauge, bar, and prereq percentages all hidden.
    expect(screen.queryByTestId('wiki-confidence')).not.toBeInTheDocument();
    expect(screen.queryByTestId('wiki-confidence-bar')).not.toBeInTheDocument();
    expect(screen.getByTestId('wiki-prereq')).toHaveTextContent('Factor');
    expect(screen.getByTestId('wiki-prereq')).not.toHaveTextContent('98%');
    // Same content-typography container as chat assistant messages.
    expect(screen.getByTestId('wiki-content')).toHaveClass('markdown', 'prose', 'message-content');
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
});
