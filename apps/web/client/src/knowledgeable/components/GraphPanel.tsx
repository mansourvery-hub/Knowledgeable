import { useCallback, useEffect, useRef, useState } from 'react';
import GraphExplorer from './GraphExplorer';
import type { Neighborhood } from '../graphTypes';
import { DEFAULT_DEPTH, DEFAULT_LIMIT } from '../graphUtils';
import {
  NeighborhoodNotFoundError,
  NeighborhoodUnavailableError,
  NeighborhoodValidationError,
  fetchNeighborhood,
  searchConcepts,
  type ConceptSearchHit,
} from '../api/graphClient';

const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/** Client-side fast feedback; the backend remains the validating authority. */
export function isConceptIdInput(value: string): boolean {
  return UUID_RE.test(value.trim());
}

function toDisplayError(err: unknown): string {
  if (err instanceof NeighborhoodNotFoundError) {
    return 'Concept not found. Check the ID and try again.';
  }
  if (err instanceof NeighborhoodUnavailableError) {
    return 'Graph service unavailable. Try again in a moment.';
  }
  if (err instanceof NeighborhoodValidationError) {
    return err.message;
  }
  if (err instanceof Error) {
    return err.message || 'Could not load the graph neighborhood.';
  }
  return 'Could not load the graph neighborhood.';
}

/**
 * Self-sufficient side-panel container for the T9 Graph Explorer.
 *
 * Takes no props so it can mount as an upstream `NavLink.Component`.
 * Owns root-concept state, bounded fetching (depth/limit), and drill-down:
 * selecting a node reloads the neighborhood centered on it. Chat remains the
 * primary surface; this panel is read-only inspection.
 */
export default function GraphPanel() {
  const [conceptId, setConceptId] = useState('');
  const [depth, setDepth] = useState(DEFAULT_DEPTH);
  const [limit, setLimit] = useState(DEFAULT_LIMIT);
  const [neighborhood, setNeighborhood] = useState<Neighborhood | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState<ConceptSearchHit[]>([]);
  const [searching, setSearching] = useState(false);
  const [searched, setSearched] = useState(false);
  const abortRef = useRef<AbortController | null>(null);

  useEffect(() => () => abortRef.current?.abort(), []);

  const load = useCallback(
    async (rootId: string, nextDepth: number, nextLimit: number) => {
      const trimmed = rootId.trim();
      if (!isConceptIdInput(trimmed)) {
        setError('Enter a valid concept UUID to load its neighborhood.');
        return;
      }
      abortRef.current?.abort();
      const controller = new AbortController();
      abortRef.current = controller;
      setLoading(true);
      setError(null);
      try {
        const result = await fetchNeighborhood({
          conceptId: trimmed,
          depth: nextDepth,
          limit: nextLimit,
          signal: controller.signal,
        });
        if (!controller.signal.aborted) {
          setNeighborhood(result);
        }
      } catch (err) {
        if (err instanceof DOMException && err.name === 'AbortError') {
          return;
        }
        if (!controller.signal.aborted) {
          setNeighborhood(null);
          setError(toDisplayError(err));
        }
      } finally {
        if (!controller.signal.aborted) {
          setLoading(false);
        }
      }
    },
    [],
  );

  const handleSelectConcept = useCallback(
    (id: string) => {
      setConceptId(id);
      void load(id, depth, limit);
    },
    [depth, limit, load],
  );

  const handleRetry = useCallback(() => {
    void load(conceptId, depth, limit);
  }, [conceptId, depth, limit, load]);

  const handleSearch = useCallback(async () => {
    const trimmed = searchQuery.trim();
    if (!trimmed) {
      setSearchResults([]);
      setSearched(false);
      return;
    }
    const controller = new AbortController();
    setSearching(true);
    try {
      const hits = await searchConcepts(trimmed, { limit: 8, signal: controller.signal });
      setSearchResults(hits);
      setSearched(true);
    } catch (err) {
      if (err instanceof DOMException && err.name === 'AbortError') {
        return;
      }
      setSearchResults([]);
      setSearched(true);
      setError(toDisplayError(err));
    } finally {
      setSearching(false);
    }
  }, [searchQuery]);

  const handlePickResult = useCallback(
    (hit: ConceptSearchHit) => {
      setSearchResults([]);
      setSearched(false);
      setConceptId(hit.id);
      void load(hit.id, depth, limit);
    },
    [depth, limit, load],
  );

  return (
    <div data-testid="graph-panel">
      <form
        data-testid="graph-search-form"
        onSubmit={(event) => {
          event.preventDefault();
          void handleSearch();
        }}
      >
        <label>
          Find a concept
          <input
            type="text"
            data-testid="graph-search-input"
            value={searchQuery}
            onChange={(event) => setSearchQuery(event.target.value)}
            placeholder="e.g. prime number"
            spellCheck={false}
          />
        </label>
        <button type="submit" data-testid="graph-search" disabled={searching}>
          {searching ? 'Searching…' : 'Search'}
        </button>
      </form>

      {searched && searchResults.length > 0 && (
        <ul data-testid="graph-search-results" aria-label="Matching concepts">
          {searchResults.map((hit) => (
            <li key={hit.id}>
              <button
                type="button"
                data-testid="graph-search-result"
                data-concept-id={hit.id}
                onClick={() => handlePickResult(hit)}
              >
                {hit.canonical_name}
              </button>
            </li>
          ))}
        </ul>
      )}
      {searched && searchResults.length === 0 && (
        <p data-testid="graph-search-empty">No concepts match that search.</p>
      )}

      <form
        data-testid="graph-root-form"
        onSubmit={(event) => {
          event.preventDefault();
          void load(conceptId, depth, limit);
        }}
      >
        <label>
          or paste a Concept ID
          <input
            type="text"
            data-testid="graph-root-input"
            value={conceptId}
            onChange={(event) => setConceptId(event.target.value)}
            placeholder="Concept UUID"
            spellCheck={false}
          />
        </label>
        <label>
          Depth
          <input
            type="number"
            data-testid="graph-depth-input"
            value={depth}
            min={0}
            max={5}
            onChange={(event) => setDepth(Number(event.target.value))}
          />
        </label>
        <label>
          Limit
          <input
            type="number"
            data-testid="graph-limit-input"
            value={limit}
            min={1}
            max={100}
            onChange={(event) => setLimit(Number(event.target.value))}
          />
        </label>
        <button type="submit" data-testid="graph-load">
          Load
        </button>
      </form>

      {!neighborhood && !loading && !error && (
        <p data-testid="graph-panel-hint">
          Search for a concept above to inspect its neighborhood. Selecting a node drills into it.
        </p>
      )}

      <GraphExplorer
        neighborhood={neighborhood}
        loading={loading}
        error={error}
        rootConceptId={conceptId}
        onSelectConcept={handleSelectConcept}
        onRetry={handleRetry}
      />
    </div>
  );
}
