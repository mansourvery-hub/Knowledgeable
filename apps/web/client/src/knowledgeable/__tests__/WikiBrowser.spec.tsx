import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import WikiBrowser from '../components/WikiBrowser';
import { closeWiki, getOpenWikiConceptId, openWiki } from '../store/wikiDrawer';
import type { MasteredList } from '../api/wikiClient';

const A = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
const B = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb';

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

function listPayload(over: Partial<MasteredList> = {}): MasteredList {
  return {
    items: [
      { id: A, name: 'Alpha', confidence: 0.82, wiki_status: 'none' },
      { id: B, name: 'Beta', confidence: 0.98, wiki_status: 'stale' },
    ],
    truncated: false,
    ...over,
  };
}

describe('WikiBrowser', () => {
  beforeEach(() => {
    global.fetch = jest.fn();
    closeWiki();
  });

  afterEach(() => {
    jest.resetAllMocks();
    closeWiki();
  });

  it('renders the Notebook title with search and skeleton rows while loading', () => {
    stubFetch(() => ({ status: 200, payload: listPayload() }));
    render(<WikiBrowser />);
    expect(screen.getByText('Notebook')).toBeInTheDocument();
    expect(screen.getByTestId('wiki-search-input')).toHaveAttribute(
      'placeholder',
      'Search your notes',
    );
    const loading = screen.getByTestId('wiki-loading');
    expect(loading).toHaveAttribute('role', 'status');
    expect(loading.querySelectorAll('.k-skeleton')).toHaveLength(3);
  });

  it('preserves server order with ring, name, and percent on every row', async () => {
    stubFetch(() => ({ status: 200, payload: listPayload() }));
    render(<WikiBrowser />);

    const rows = await screen.findAllByTestId('wiki-row');
    expect(rows).toHaveLength(2);
    // Weakest-first server order is never re-sorted client-side.
    expect(rows[0]).toHaveTextContent('Alpha');
    expect(rows[1]).toHaveTextContent('Beta');
    // Percentages are always visible (redesign copy deck, not debug-gated).
    const badges = screen.getAllByTestId('wiki-confidence');
    expect(badges).toHaveLength(2);
    expect(badges[0]).toHaveTextContent('82%');
    expect(badges[1]).toHaveTextContent('98%');
    expect(screen.getByTestId('wiki-stale-mark')).toHaveTextContent('May be outdated');
  });

  it('marks the row whose concept the drawer shows', async () => {
    stubFetch(() => ({ status: 200, payload: listPayload() }));
    openWiki(B);
    render(<WikiBrowser />);

    const rows = await screen.findAllByTestId('wiki-row');
    expect(rows[0]).not.toHaveAttribute('aria-current');
    expect(rows[1]).toHaveAttribute('aria-current', 'true');
  });

  it('narrows the list by name without re-sorting', async () => {
    stubFetch(() => ({ status: 200, payload: listPayload() }));
    render(<WikiBrowser />);
    await screen.findAllByTestId('wiki-row');

    fireEvent.change(screen.getByTestId('wiki-search-input'), { target: { value: 'alp' } });

    const rows = screen.getAllByTestId('wiki-row');
    expect(rows).toHaveLength(1);
    expect(rows[0]).toHaveTextContent('Alpha');

    fireEvent.change(screen.getByTestId('wiki-search-input'), { target: { value: 'zzz' } });
    expect(screen.getByTestId('wiki-no-match')).toHaveTextContent('No notes match that search.');
  });

  it('opens the drawer with the row concept id', async () => {
    stubFetch(() => ({ status: 200, payload: listPayload() }));
    render(<WikiBrowser />);
    const rows = await screen.findAllByTestId('wiki-row');

    fireEvent.click(rows[1]);

    expect(getOpenWikiConceptId()).toBe(B);
  });

  it('renders the empty state when nothing is mastered', async () => {
    stubFetch(() => ({ status: 200, payload: listPayload({ items: [] }) }));
    render(<WikiBrowser />);

    expect(await screen.findByTestId('wiki-empty')).toHaveTextContent(
      'No notes yet. Pages appear once you understand a concept well.',
    );
    expect(screen.queryByTestId('wiki-list')).not.toBeInTheDocument();
  });

  it('renders the error state with a working retry', async () => {
    stubFetch(() => ({ status: 503, payload: { message: 'down' } }));
    render(<WikiBrowser />);
    expect(await screen.findByTestId('wiki-error')).toHaveTextContent(
      "Your notebook isn't available right now.",
    );

    (global.fetch as jest.Mock).mockReset();
    stubFetch(() => ({ status: 200, payload: listPayload() }));
    fireEvent.click(screen.getByTestId('wiki-retry'));

    await waitFor(() => expect(screen.getAllByTestId('wiki-row')).toHaveLength(2));
  });

  it('notes truncation without hiding the loaded rows', async () => {
    stubFetch(() => ({ status: 200, payload: listPayload({ truncated: true }) }));
    render(<WikiBrowser />);

    await screen.findAllByTestId('wiki-row');
    expect(screen.getByTestId('wiki-truncated-note')).toHaveTextContent(
      'Showing the 2 weakest. Use the map to find others.',
    );
  });
});
