import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import GraphPanel, { isConceptIdInput } from '../components/GraphPanel';
import type { Neighborhood } from '../graphTypes';

const A = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
const B = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb';

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

function stubFetch(
  impl: (url: string) => { status: number; payload: unknown },
) {
  (global.fetch as jest.Mock).mockImplementation((url: string) => {
    const { status, payload } = impl(url);
    return Promise.resolve({
      ok: status >= 200 && status < 300,
      status,
      json: async () => payload,
    } as Response);
  });
}

describe('GraphPanel', () => {
  beforeEach(() => {
    global.fetch = jest.fn();
  });

  afterEach(() => {
    jest.resetAllMocks();
  });

  it('validates UUID shape on the client', () => {
    expect(isConceptIdInput(A)).toBe(true);
    expect(isConceptIdInput('  not-a-uuid ')).toBe(false);
    expect(isConceptIdInput('')).toBe(false);
  });

  it('prompts for a root concept without fetching on mount', () => {
    render(<GraphPanel />);
    expect(screen.getByTestId('graph-panel-hint')).toBeInTheDocument();
    expect(global.fetch).not.toHaveBeenCalled();
  });

  it('rejects a malformed ID without fetching', () => {
    render(<GraphPanel />);
    fireEvent.change(screen.getByTestId('graph-root-input'), {
      target: { value: 'nope' },
    });
    fireEvent.click(screen.getByTestId('graph-load'));
    expect(screen.getByRole('alert')).toHaveTextContent('valid concept UUID');
    expect(global.fetch).not.toHaveBeenCalled();
  });

  it('loads and renders the neighborhood for a valid ID', async () => {
    stubFetch(() => ({ status: 200, payload: neighborhoodFor(A, 'Alpha') }));
    render(<GraphPanel />);
    fireEvent.change(screen.getByTestId('graph-root-input'), {
      target: { value: A },
    });
    fireEvent.click(screen.getByTestId('graph-load'));

    expect(screen.getByTestId('graph-loading')).toBeInTheDocument();
    await waitFor(() => {
      expect(screen.getByTestId('graph-counts')).toHaveTextContent('2 concepts');
    });
    const url = (global.fetch as jest.Mock).mock.calls[0][0] as string;
    expect(url).toContain(`concept_id=${A}`);
  });

  it('drills down when a node is selected', async () => {
    stubFetch((url) => {
      const id = new URL(url, 'http://localhost').searchParams.get('concept_id');
      return {
        status: 200,
        payload: neighborhoodFor(id ?? A, id === B ? 'Beta' : 'Alpha'),
      };
    });
    render(<GraphPanel />);
    fireEvent.change(screen.getByTestId('graph-root-input'), {
      target: { value: A },
    });
    fireEvent.click(screen.getByTestId('graph-load'));
    await waitFor(() => {
      expect(screen.getByTestId('graph-counts')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByTestId(`graph-node-select-${B}`));
    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledTimes(2);
    });
    const secondUrl = (global.fetch as jest.Mock).mock.calls[1][0] as string;
    expect(secondUrl).toContain(`concept_id=${B}`);
    expect(
      (screen.getByTestId('graph-root-input') as HTMLInputElement).value,
    ).toBe(B);
  });

  it('maps 404 to a not-found message and 503 to unavailable', async () => {
    stubFetch(() => ({ status: 404, payload: { code: 'not_found', message: 'gone' } }));
    render(<GraphPanel />);
    fireEvent.change(screen.getByTestId('graph-root-input'), {
      target: { value: A },
    });
    fireEvent.click(screen.getByTestId('graph-load'));
    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('not found');
    });

    (global.fetch as jest.Mock).mockReset();
    stubFetch(() => ({
      status: 503,
      payload: { code: 'service_unavailable', message: 'down' },
    }));
    fireEvent.click(screen.getByTestId('graph-retry'));
    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('unavailable');
    });
  });

  it('searches by name and loads the picked concept', async () => {
    stubFetch((url) => {
      if (url.startsWith('/api/concepts/search')) {
        return {
          status: 200,
          payload: [
            { id: A, canonical_name: 'Alpha', canonical_statement: 'Alpha statement.' },
          ],
        };
      }
      return { status: 200, payload: neighborhoodFor(A, 'Alpha') };
    });
    render(<GraphPanel />);
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
    expect(calls[0]).toContain('/api/concepts/search?q=alpha');
    expect(calls[1]).toContain(`/api/graph/neighborhood?concept_id=${A}`);
  });

  it('shows an empty state when search matches nothing', async () => {
    stubFetch(() => ({ status: 200, payload: [] }));
    render(<GraphPanel />);
    fireEvent.change(screen.getByTestId('graph-search-input'), {
      target: { value: 'zzz' },
    });
    fireEvent.click(screen.getByTestId('graph-search'));
    await waitFor(() => {
      expect(screen.getByTestId('graph-search-empty')).toBeInTheDocument();
    });
  });
});
