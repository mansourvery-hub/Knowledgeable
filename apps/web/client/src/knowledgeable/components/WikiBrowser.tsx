import { useCallback, useEffect, useRef, useState } from 'react';
import { useOpenWikiConceptId, openWiki } from '../store/wikiDrawer';
import {
  fetchMasteredConcepts,
  WikiUnavailableError,
  WikiValidationError,
  type MasteredConceptItem,
} from '../api/wikiClient';
import SearchField from './ui/SearchField';
import ConceptRow from './ui/ConceptRow';
import EmptyState from './ui/EmptyState';
import ErrorState from './ui/ErrorState';

/** Copy deck (SPEC 9): learner-facing notebook errors. */
function toDisplayError(err: unknown): string {
  if (err instanceof WikiUnavailableError) {
    return "Your wiki isn't available right now. Try again in a moment.";
  }
  if (err instanceof WikiValidationError) {
    return err.message;
  }
  if (err instanceof Error) {
    return err.message || 'Could not load your wiki.';
  }
  return 'Could not load your wiki.';
}

/**
 * Notebook panel (Phase 3, SPEC 7.1).
 *
 * Self-sufficient side-panel container: loads the bounded mastered-concepts
 * list once (server order is weakest-first and is never re-sorted here),
 * filters names locally, and opens the reading pane via `openWiki`.
 */
export default function WikiBrowser() {
  const [items, setItems] = useState<MasteredConceptItem[] | null>(null);
  const [truncated, setTruncated] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState('');
  const abortRef = useRef<AbortController | null>(null);
  // Active-row highlight follows the open drawer, like the chat history
  // marks the open conversation.
  const openConceptId = useOpenWikiConceptId();

  useEffect(() => () => abortRef.current?.abort(), []);

  const load = useCallback(async () => {
    abortRef.current?.abort();
    const controller = new AbortController();
    abortRef.current = controller;
    setLoading(true);
    setError(null);
    try {
      const result = await fetchMasteredConcepts({ signal: controller.signal });
      if (!controller.signal.aborted) {
        setItems(result.items);
        setTruncated(result.truncated);
      }
    } catch (err) {
      if (err instanceof DOMException && err.name === 'AbortError') {
        return;
      }
      if (!controller.signal.aborted) {
        setItems(null);
        setError(toDisplayError(err));
      }
    } finally {
      if (!controller.signal.aborted) {
        setLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const query = filter.trim().toLowerCase();
  const visible =
    items == null ? [] : query ? items.filter((item) => item.name.toLowerCase().includes(query)) : items;

  return (
    <section className="k-panel" data-testid="wiki-browser" aria-label="Wiki">
      <h2 className="k-panel__title">Wiki</h2>
      <SearchField
        value={filter}
        onChange={setFilter}
        placeholder="Search your notes"
        ariaLabel="Search your notes"
        testId="wiki-search-input"
      />

      {loading && (
        <div role="status" data-testid="wiki-loading">
          <span className="k-sr">Loading notes…</span>
          <div className="k-skeleton" />
          <div className="k-skeleton" />
          <div className="k-skeleton" />
        </div>
      )}

      {!loading && error && (
        <ErrorState
          message={error}
          onRetry={() => void load()}
          retryLabel="Try again"
          testId="wiki-error"
          retryTestId="wiki-retry"
        />
      )}

      {!loading && !error && items != null && items.length === 0 && (
        <EmptyState
          message="No notes yet. Pages appear once you understand a concept well."
          testId="wiki-empty"
        />
      )}

      {!loading && !error && items != null && items.length > 0 && visible.length === 0 && (
        <EmptyState message="No notes match that search." testId="wiki-no-match" />
      )}

      {!loading && !error && visible.length > 0 && (
        <ul className="k-rows" data-testid="wiki-list" aria-label="Mastered concepts">
          {visible.map((item) => (
            <ConceptRow
              key={item.id}
              name={item.name}
              value={item.confidence}
              selected={openConceptId === item.id}
              sub={item.wiki_status === 'stale' ? 'May be outdated' : undefined}
              onSelect={() => openWiki(item.id)}
              testId="wiki-row"
              pctTestId="wiki-confidence"
              subTestId="wiki-stale-mark"
            />
          ))}
        </ul>
      )}

      {!loading && !error && truncated && (
        <p data-testid="wiki-truncated-note">
          Showing the {items?.length ?? 0} weakest. Use the concept map to find others.
        </p>
      )}
    </section>
  );
}
