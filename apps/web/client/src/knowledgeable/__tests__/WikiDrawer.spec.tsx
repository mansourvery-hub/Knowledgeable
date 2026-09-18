import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import WikiDrawer from '../components/WikiDrawer';

jest.mock('~/components/Chat/Messages/Content/MarkdownBlocks', () => ({
  __esModule: true,
  default: ({ content }: { content: string }) => (
    <div data-testid="wiki-markdown-stub">{content}</div>
  ),
}));

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
    render(<WikiDrawer conceptId={null} />);
    expect(screen.queryByTestId('wiki-drawer')).not.toBeInTheDocument();
    expect(global.fetch).not.toHaveBeenCalled();
  });

  it('loads and renders the cached page with anchors and gauge', async () => {
    const onClose = jest.fn();
    stubFetch(200, pagePayload());
    render(<WikiDrawer conceptId={CONCEPT} onClose={onClose} />);

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
    const { unmount } = render(<WikiDrawer conceptId={CONCEPT} />);
    await waitFor(() => {
      expect(screen.getByTestId('wiki-stale')).toBeInTheDocument();
    });
    unmount();

    (global.fetch as jest.Mock).mockReset();
    stubFetch(404, { code: 'wiki_not_ready', message: 'keep learning' });
    render(<WikiDrawer conceptId={CONCEPT} />);
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
});
