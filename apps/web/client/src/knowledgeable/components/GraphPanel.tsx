import { useCallback, useEffect, useRef, useState } from 'react';
import MapCanvas from './MapCanvas';
import SearchField from './ui/SearchField';
import Button from './ui/Button';
import Segmented from './ui/Segmented';
import ConceptRow from './ui/ConceptRow';
import EmptyState from './ui/EmptyState';
import ErrorState from './ui/ErrorState';
import ConfidenceRing from './ui/ConfidenceRing';
import type { Neighborhood } from '../graphTypes';
import {
  DEFAULT_DEPTH,
  DEFAULT_LIMIT,
  confidenceWords,
  countNeighborhood,
  filterReviewOnly,
  formatConfidence,
  isReviewEligible,
  sortByConfidenceAscending,
} from '../graphUtils';
import {
  NeighborhoodNotFoundError,
  NeighborhoodUnavailableError,
  NeighborhoodValidationError,
  fetchNeighborhood,
  searchConcepts,
  type ConceptSearchHit,
} from '../api/graphClient';
import { fetchMasteredConcepts } from '../api/wikiClient';
import { getSelectedConceptId, setSelectedConceptId } from '../store/mapSelection';
import { openWiki } from '../store/wikiDrawer';

const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/** Client-side fast feedback; the backend remains the validating authority. */
export function isConceptIdInput(value: string): boolean {
  return UUID_RE.test(value.trim());
}

/** Copy deck (SPEC 9): learner-facing map errors. */
function toDisplayError(err: unknown): string {
  if (err instanceof NeighborhoodNotFoundError) {
    return "We couldn't find that concept.";
  }
  if (err instanceof NeighborhoodUnavailableError) {
    return "The concept map isn't available right now. Try again in a moment.";
  }
  if (err instanceof NeighborhoodValidationError) {
    return err.message;
  }
  if (err instanceof Error) {
    return err.message || "Couldn't load the concept map. Try again.";
  }
  return "Couldn't load the concept map. Try again.";
}

function showDevControls(): boolean {
  if (import.meta.env.DEV) {
    return true;
  }
  if (typeof window !== 'undefined') {
    return new URLSearchParams(window.location.search).has('kdebug');
  }
  return false;
}

type ReviewFilter = 'all' | 'review';

/**
 * Map panel (Phase 2, SPEC 6.1): title, search, focus summary, canvas,
 * legend, segmented filter, and weakest-first rows. Data logic (abort
 * controllers, load, search, drill-down) is unchanged from the T9 panel.
 */
export default function GraphPanel() {
  const [conceptId, setConceptId] = useState('');
  const [depth, setDepth] = useState(DEFAULT_DEPTH);
  const [limit, setLimit] = useState(DEFAULT_LIMIT);
  const [neighborhood, setNeighborhood] = useState<Neighborhood | null>(null);
  const [loading, setLoading] = useState(false);
  const [booting, setBooting] = useState(true);
  const [bootEmpty, setBootEmpty] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState<ConceptSearchHit[]>([]);
  const [searching, setSearching] = useState(false);
  const [searched, setSearched] = useState(false);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [filter, setFilter] = useState<ReviewFilter>('all');
  const abortRef = useRef<AbortController | null>(null);

  useEffect(() => () => abortRef.current?.abort(), []);

  const load = useCallback(
    async (rootId: string, nextDepth: number, nextLimit: number) => {
      const trimmed = rootId.trim();
      if (!isConceptIdInput(trimmed)) {
        setError("We couldn't find that concept.");
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
  const loadRef = useRef(load);
  loadRef.current = load;

  // Default state (SPEC 6.2): saved selection, else weakest mastered concept,
  // else the nothing-mapped empty state. A mastered-list failure degrades to
  // the empty state; search stays available.
  useEffect(() => {
    let cancelled = false;
    const controller = new AbortController();
    const boot = async () => {
      const saved = getSelectedConceptId();
      if (saved) {
        setSelectedId(saved);
        setBooting(false);
        await loadRef.current(saved, DEFAULT_DEPTH, DEFAULT_LIMIT);
        return;
      }
      try {
        const list = await fetchMasteredConcepts({ signal: controller.signal });
        if (cancelled || controller.signal.aborted) {
          return;
        }
        const first = list.items[0];
        if (first) {
          setSelectedConceptId(first.id);
          setSelectedId(first.id);
          setBooting(false);
          await loadRef.current(first.id, DEFAULT_DEPTH, DEFAULT_LIMIT);
        } else {
          setBootEmpty(true);
          setBooting(false);
        }
      } catch {
        if (!cancelled) {
          setBootEmpty(true);
          setBooting(false);
        }
      }
    };
    void boot();
    return () => {
      cancelled = true;
      controller.abort();
    };
  }, []);

  const handleSelectConcept = useCallback(
    (id: string) => {
      setSelectedConceptId(id);
      setSelectedId(id);
      setConceptId(id);
      void load(id, depth, limit);
    },
    [depth, limit, load],
  );

  const handleRetry = useCallback(() => {
    if (selectedId) {
      void load(selectedId, depth, limit);
    } else {
      void load(conceptId, depth, limit);
    }
  }, [conceptId, depth, limit, load, selectedId]);

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
      // The panel shows at most 8 results; slice defensively so a backend
      // over-delivery can never widen the list.
      const hits = (await searchConcepts(trimmed, { limit: 8, signal: controller.signal })).slice(0, 8);
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
      setSearchQuery('');
      setSearchResults([]);
      setSearched(false);
      handleSelectConcept(hit.id);
    },
    [handleSelectConcept],
  );

  const sorted = sortByConfidenceAscending(neighborhood?.nodes ?? []);
  const counts = countNeighborhood(sorted, neighborhood?.edges ?? []);
  const visibleRows = filter === 'review' ? filterReviewOnly(sorted) : sorted;
  // The canvas honors the review filter but always keeps the focus concept.
  const focusNode =
    sorted.find((n) => n.concept.id === selectedId) ?? neighborhood?.nodes[0] ?? null;
  const canvasNodes =
    filter === 'review' && focusNode
      ? sorted.filter((n) => n.concept.id === focusNode.concept.id || isReviewEligible(n))
      : sorted;
  const focusConfidence = focusNode?.learner_confidence ?? null;
  const showResults = searched && searchQuery.trim() !== '';
  const showSkeleton = (loading || booting) && neighborhood == null;

  return (
    <section className="k-panel" data-testid="graph-panel" aria-label="Concept Map">
      <h2 className="k-panel__title">Concept Map</h2>

      <form
        data-testid="graph-search-form"
        onSubmit={(event) => {
          event.preventDefault();
          void handleSearch();
        }}
      >
        <SearchField
          value={searchQuery}
          onChange={setSearchQuery}
          placeholder="Find a concept"
          ariaLabel="Find a concept"
          testId="graph-search-input"
          onSubmit={() => void handleSearch()}
        />
      </form>

      {showResults ? (
        searchResults.length > 0 ? (
          <ul className="k-rows" data-testid="graph-search-results" aria-label="Matching concepts">
            {searchResults.map((hit) => (
              <li key={hit.id}>
                <button
                  type="button"
                  className="k-row"
                  data-testid="graph-search-result"
                  data-concept-id={hit.id}
                  onClick={() => handlePickResult(hit)}
                >
                  <span className="k-row__name">{hit.canonical_name}</span>
                </button>
              </li>
            ))}
          </ul>
        ) : (
          <EmptyState message="No concepts match that search." testId="graph-search-empty" />
        )
      ) : (
        focusNode && (
          <div className="k-focus">
            <ConfidenceRing value={focusConfidence} size={34} />
            <div>
              <div className="k-focus__name">{focusNode.concept.canonical_name}</div>
              <div className="k-focus__state">
                {confidenceWords(focusConfidence)}, {formatConfidence(focusConfidence)}
              </div>
            </div>
            <Button onClick={() => openWiki(focusNode.concept.id)}>Open notes</Button>
          </div>
        )
      )}

      {showSkeleton && (
        <div role="status" data-testid="graph-loading">
          <div className="k-skeleton" />
          <div className="k-skeleton" />
          <div className="k-skeleton k-skeleton--short" />
        </div>
      )}

      {error && (
        <ErrorState
          message={error}
          onRetry={handleRetry}
          retryLabel="Try again"
          testId="graph-error"
          retryTestId="graph-retry"
        />
      )}

      {!neighborhood && !loading && !booting && !error && (
        <EmptyState
          message="Nothing mapped yet. Ask the tutor about a topic and it will appear here."
          testId="graph-empty"
        />
      )}

      {neighborhood && (
        <>
          <MapCanvas
            rootId={focusNode?.concept.id ?? ''}
            nodes={canvasNodes}
            edges={neighborhood.edges}
            selectedId={focusNode?.concept.id}
            onSelectConcept={handleSelectConcept}
          />
          <div className="k-legend">Arrows point to what a concept builds on.</div>
          <Segmented
            allCount={counts.concepts}
            reviewCount={counts.needReview}
            selected={filter}
            onSelect={setFilter}
          />
          <ul className="k-rows" data-testid="graph-nodes" aria-label="Concepts">
            {visibleRows.map((node) => (
              <ConceptRow
                key={node.concept.id}
                name={node.concept.canonical_name}
                value={node.learner_confidence}
                selected={node.concept.id === focusNode?.concept.id}
                onSelect={() => handleSelectConcept(node.concept.id)}
                testId={`graph-node-select-${node.concept.id}`}
              />
            ))}
          </ul>
        </>
      )}

      {showDevControls() && (
        <details className="k-dev">
          <summary>Developer controls</summary>
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
        </details>
      )}

      <p className="k-sr" data-testid="graph-counts">
        {counts.concepts} concepts · {counts.links} links · {counts.needReview} need review
      </p>
    </section>
  );
}
