import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import GraphPanel, { isConceptIdInput } from '../components/GraphPanel';
import { getSelectedConceptId, setSelectedConceptId } from '../store/mapSelection';
import { closeWiki, getOpenWikiConceptId } from '../store/wikiDrawer';
import type { Neighborhood } from '../graphTypes';

const A = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
const B = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb';
const C = 'cccccccc-cccc-cccc-cccc-cccccccccccc';

function node(id: string, name: string, learnerConfidence: number | null) {
  return {
    concept: {
      id,
      canonical_name: name,
      canonical_statement: `${name} statement.`,
      learner_statement: null,
      world_confidence: 1.0,
      status: 'Active' as const,
      created_at: '2026-01-01T00:00:00Z',
      updated_at: '2026-01-01T00:00:00Z',
    },
    learner_confidence: learnerConfidence,
    is_healthy: learnerConfidence == null ? null : learnerConfidence >= 0.95,
    is_review_eligible: learnerConfidence == null ? null : learnerConfidence < 0.95,
  };
}

function neighborhoodFor(rootId: string, rootName: string): Neighborhood {
  return {
    nodes: [node(rootId, rootName, 0.3), node(B, 'Beta', 0.98)],
    edges: [
      {
        id: 'e1',
        from_concept_id: rootId,
        to_concept_id: B,
        relation_type: 'Dependency',
        created_at: '2026-01-01T00:00:00Z',
        updated_at: '2026-01-01T00:00:00Z',
      },
    ],
  };
}

function masteredItems(items: Array<{ id: string; name: string; confidence: number }> = []) {
  return {
    items: items.map((item) => ({ ...item, wiki_status: 'ready' as const })),
    truncated: false,
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

function neighborhoodCalls(): string[] {
  return (global.fetch as jest.Mock).mock.calls
    .map((c) => c[0] as string)
    .filter((url) => url.includes('/api/graph/neighborhood'));
}

describe('GraphPanel', () => {
  beforeEach(() => {
    global.fetch = jest.fn();
    setSelectedConceptId(null);
    closeWiki();
  });

  afterEach(() => {
    jest.resetAllMocks();
    setSelectedConceptId(null);
    closeWiki();
  });

  it('validates UUID shape on the client', () => {
    expect(isConceptIdInput(A)).toBe(true);
    expect(isConceptIdInput('  not-a-uuid ')).toBe(false);
    expect(isConceptIdInput('')).toBe(false);
  });

  it('loads the saved session concept first on mount', async () => {
    stubFetch((url) => {
      if (url.startsWith('/api/concepts/mastered')) {
        return { status: 200, payload: masteredItems() };
      }
      return { status: 200, payload: neighborhoodFor(A, 'Alpha') };
    });
    setSelectedConceptId(A);
    render(<GraphPanel />);
    await waitFor(() => {
      expect(screen.getByTestId('graph-counts')).toHaveTextContent('2 concepts');
    });
    const calls = neighborhoodCalls();
    expect(calls).toHaveLength(1);
    expect(calls[0]).toContain(`concept_id=${A}`);
    expect(global.fetch).not.toHaveBeenCalledWith(
      expect.stringContaining('/api/concepts/mastered'),
      expect.anything(),
    );
    expect(getSelectedConceptId()).toBe(A);
  });

  it('falls back to the weakest mastered concept when nothing was selected', async () => {
    stubFetch((url) => {
      if (url.startsWith('/api/concepts/mastered')) {
        return { status: 200, payload: masteredItems([{ id: A, name: 'Alpha', confidence: 0.3 }]) };
      }
      return { status: 200, payload: neighborhoodFor(A, 'Alpha') };
    });
    render(<GraphPanel />);
    await waitFor(() => {
      expect(screen.getByTestId('graph-counts')).toHaveTextContent('2 concepts');
    });
    expect(neighborhoodCalls()[0]).toContain(`concept_id=${A}`);
    expect(getSelectedConceptId()).toBe(A);
  });

  it('shows the nothing-mapped empty state when there is nothing to load', async () => {
    stubFetch(() => ({ status: 200, payload: masteredItems() }));
    render(<GraphPanel />);
    await waitFor(() => {
      expect(screen.getByTestId('graph-empty')).toHaveTextContent('Nothing mapped yet.');
    });
    expect(neighborhoodCalls()).toHaveLength(0);
  });

  it('rejects a malformed ID without fetching the neighborhood', async () => {
    stubFetch(() => ({ status: 200, payload: masteredItems() }));
    render(<GraphPanel />);
    await waitFor(() => {
      expect(screen.getByTestId('graph-empty')).toBeInTheDocument();
    });
    fireEvent.change(screen.getByTestId('graph-root-input'), {
      target: { value: 'nope' },
    });
    fireEvent.click(screen.getByTestId('graph-load'));
    expect(screen.getByRole('alert')).toHaveTextContent("We couldn't find that concept.");
    expect(neighborhoodCalls()).toHaveLength(0);
  });

  it('loads and renders the neighborhood for a valid ID', async () => {
    stubFetch((url) => {
      if (url.startsWith('/api/concepts/mastered')) {
        return { status: 200, payload: masteredItems() };
      }
      return { status: 200, payload: neighborhoodFor(A, 'Alpha') };
    });
    render(<GraphPanel />);
    await waitFor(() => {
      expect(screen.getByTestId('graph-empty')).toBeInTheDocument();
    });
    fireEvent.change(screen.getByTestId('graph-root-input'), {
      target: { value: A },
    });
    fireEvent.click(screen.getByTestId('graph-load'));

    expect(screen.getByTestId('graph-loading')).toBeInTheDocument();
    await waitFor(() => {
      expect(screen.getByTestId('graph-counts')).toHaveTextContent('2 concepts');
    });
    const url = neighborhoodCalls()[0];
    expect(url).toContain(`concept_id=${A}`);
  });

  it('drills down when a row is selected', async () => {
    stubFetch((url) => {
      if (url.startsWith('/api/concepts/mastered')) {
        return { status: 200, payload: masteredItems() };
      }
      const id = new URL(url, 'http://localhost').searchParams.get('concept_id');
      return {
        status: 200,
        payload: neighborhoodFor(id ?? A, id === B ? 'Beta' : 'Alpha'),
      };
    });
    render(<GraphPanel />);
    await waitFor(() => {
      expect(screen.getByTestId('graph-empty')).toBeInTheDocument();
    });
    fireEvent.change(screen.getByTestId('graph-root-input'), {
      target: { value: A },
    });
    fireEvent.click(screen.getByTestId('graph-load'));
    await waitFor(() => {
      expect(screen.getByTestId('graph-counts')).toHaveTextContent('2 concepts');
    });

    fireEvent.click(screen.getByTestId(`graph-node-select-${B}`));
    await waitFor(() => {
      expect(neighborhoodCalls()).toHaveLength(2);
    });
    expect(neighborhoodCalls()[1]).toContain(`concept_id=${B}`);
    expect((screen.getByTestId('graph-root-input') as HTMLInputElement).value).toBe(B);
  });

  it('maps 404 to not-found copy and 503 to unavailable copy', async () => {
    stubFetch((url) => {
      if (url.startsWith('/api/concepts/mastered')) {
        return { status: 200, payload: masteredItems() };
      }
      return { status: 404, payload: { code: 'not_found', message: 'gone' } };
    });
    render(<GraphPanel />);
    await waitFor(() => {
      expect(screen.getByTestId('graph-empty')).toBeInTheDocument();
    });
    fireEvent.change(screen.getByTestId('graph-root-input'), {
      target: { value: A },
    });
    fireEvent.click(screen.getByTestId('graph-load'));
    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent("We couldn't find that concept.");
    });

    (global.fetch as jest.Mock).mockReset();
    stubFetch(() => ({
      status: 503,
      payload: { code: 'service_unavailable', message: 'down' },
    }));
    fireEvent.click(screen.getByTestId('graph-retry'));
    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent("The map isn't available right now.");
    });
  });

  it('searches by name and loads the picked concept', async () => {
    stubFetch((url) => {
      if (url.startsWith('/api/concepts/mastered')) {
        return { status: 200, payload: masteredItems() };
      }
      if (url.startsWith('/api/concepts/search')) {
        return {
          status: 200,
          payload: [{ id: A, canonical_name: 'Alpha', canonical_statement: 'Alpha statement.' }],
        };
      }
      return { status: 200, payload: neighborhoodFor(A, 'Alpha') };
    });
    render(<GraphPanel />);
    await waitFor(() => {
      expect(screen.getByTestId('graph-empty')).toBeInTheDocument();
    });
    fireEvent.change(screen.getByTestId('graph-search-input'), {
      target: { value: 'alpha' },
    });
    fireEvent.click(screen.getByTestId('graph-search'));
    await waitFor(() => {
      expect(screen.getByTestId('graph-search-result')).toHaveTextContent('Alpha');
    });
    fireEvent.click(screen.getByTestId('graph-search-result'));
    await waitFor(() => {
      expect(screen.getByTestId('graph-counts')).toHaveTextContent('2 concepts');
    });
    const calls = (global.fetch as jest.Mock).mock.calls.map((c) => c[0] as string);
    expect(calls[0]).toContain('/api/concepts/mastered');
    expect(calls[1]).toContain('/api/concepts/search?q=alpha');
    expect(calls[2]).toContain(`/api/graph/neighborhood?concept_id=${A}`);
  });

  it('shows an empty state when search matches nothing', async () => {
    stubFetch((url) => {
      if (url.startsWith('/api/concepts/mastered')) {
        return { status: 200, payload: masteredItems() };
      }
      return { status: 200, payload: [] };
    });
    render(<GraphPanel />);
    await waitFor(() => {
      expect(screen.getByTestId('graph-empty')).toBeInTheDocument();
    });
    fireEvent.change(screen.getByTestId('graph-search-input'), {
      target: { value: 'zzz' },
    });
    fireEvent.click(screen.getByTestId('graph-search'));
    await waitFor(() => {
      expect(screen.getByTestId('graph-search-empty')).toBeInTheDocument();
    });
  });

  it('lists concepts weakest first with counts and the focus summary', async () => {
    stubFetch((url) => {
      if (url.startsWith('/api/concepts/mastered')) {
        return { status: 200, payload: masteredItems([{ id: A, name: 'Alpha', confidence: 0.3 }]) };
      }
      return {
        status: 200,
        payload: {
          nodes: [node(A, 'Alpha', 0.98), node(B, 'Beta', 0.3), node(C, 'Gamma', null)],
          edges: [],
        },
      };
    });
    render(<GraphPanel />);
    await waitFor(() => {
      expect(screen.getByTestId('graph-counts')).toHaveTextContent(
        '3 concepts · 0 links · 1 need review',
      );
    });
    // Weakest known confidence sorts first; unseen sorts last.
    const rows = screen.getAllByTestId(/^graph-node-select-/);
    expect(rows.map((r) => r.getAttribute('data-testid'))).toEqual([
      `graph-node-select-${B}`,
      `graph-node-select-${A}`,
      `graph-node-select-${C}`,
    ]);
    expect(rows[0]).toHaveTextContent('30%');
    // Focus summary names the selected concept with its words.
    expect(screen.getByText('Solid, 98%')).toBeInTheDocument();
  });

  it('filters to needs-review rows with the segmented control', async () => {
    stubFetch((url) => {
      if (url.startsWith('/api/concepts/mastered')) {
        return { status: 200, payload: masteredItems([{ id: A, name: 'Alpha', confidence: 0.3 }]) };
      }
      return {
        status: 200,
        payload: {
          nodes: [node(A, 'Alpha', 0.98), node(B, 'Beta', 0.3)],
          edges: [],
        },
      };
    });
    render(<GraphPanel />);
    await waitFor(() => {
      expect(screen.getByTestId('graph-counts')).toHaveTextContent('2 concepts');
    });
    expect(screen.getByRole('button', { name: 'All 2' })).toHaveAttribute('aria-pressed', 'true');
    fireEvent.click(screen.getByRole('button', { name: 'Needs review 1' }));
    const rows = screen.getAllByTestId(/^graph-node-select-/);
    expect(rows).toHaveLength(1);
    expect(rows[0]).toHaveAttribute('data-testid', `graph-node-select-${B}`);
  });

  it('keeps the focus concept on the canvas under the review filter', async () => {
    stubFetch((url) => {
      if (url.startsWith('/api/concepts/mastered')) {
        return { status: 200, payload: masteredItems([{ id: A, name: 'Alpha', confidence: 0.3 }]) };
      }
      return {
        status: 200,
        payload: {
          nodes: [node(A, 'Alpha', 0.98), node(B, 'Beta', 0.3)],
          edges: [],
        },
      };
    });
    render(<GraphPanel />);
    await waitFor(() => {
      expect(screen.getByTestId('graph-counts')).toHaveTextContent('2 concepts');
    });
    fireEvent.click(screen.getByRole('button', { name: 'Needs review 1' }));
    // Rows narrow to Beta; the canvas keeps Beta plus the Alpha focus.
    expect(screen.getAllByTestId(/^graph-node-select-/)).toHaveLength(1);
    expect(screen.getAllByTestId('graph-canvas-node')).toHaveLength(2);
  });

  it('opens notes for the focus concept', async () => {
    stubFetch((url) => {
      if (url.startsWith('/api/concepts/mastered')) {
        return { status: 200, payload: masteredItems([{ id: A, name: 'Alpha', confidence: 0.3 }]) };
      }
      return { status: 200, payload: neighborhoodFor(A, 'Alpha') };
    });
    render(<GraphPanel />);
    await waitFor(() => {
      expect(screen.getByTestId('graph-counts')).toHaveTextContent('2 concepts');
    });
    fireEvent.click(screen.getByRole('button', { name: 'Open notes' }));
    expect(getOpenWikiConceptId()).toBe(A);
  });
});
