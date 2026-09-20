import { useCallback, useEffect, useRef, useState } from 'react';
import { openWiki } from '../store/wikiDrawer';
import {
  fetchMasteredConcepts,
  WikiUnavailableError,
  WikiValidationError,
  type MasteredConceptItem,
} from '../api/wikiClient';
import { formatConfidence } from '../graphUtils';

function toDisplayError(err: unknown): string {
  if (err instanceof WikiUnavailableError) {
    return 'Wiki service unavailable. Try again in a moment.';
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
 * Self-sufficient side-panel container for the Phase 5a wiki browser.
 *
 * Takes no props so it can mount as an upstream `NavLink.Component`. Loads
 * the bounded mastered-concepts list once (server order is weakest-first and
 * is never re-sorted here), filters names locally, and opens the existing
 * drawer via `openWiki` — the drawer itself is untouched. Chat remains the
 * primary surface; this panel is read-only browsing.
 */
export default function WikiBrowser() {
  const [items, setItems] = useState<MasteredConceptItem[] | null>(null);
  const [truncated, setTruncated] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState('');
  const abortRef = useRef<AbortController | null>(null);

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
  const visible = items == null ? [] : query ? items.filter((item) => item.name.toLowerCase().includes(query)) : items;

  return (
    <div data-testid="wiki-browser">
      <label>
        Find in your wiki
        <input
          type="text"
          data-testid="wiki-search-input"
          value={filter}
          onChange={(event) => setFilter(event.target.value)}
          placeholder="e.g. prime number"
          spellCheck={false}
        />
      </label>

      {loading && <p data-testid="wiki-loading">Loading your wiki…</p>}

      {!loading && error && (
        <div>
          <p data-testid="wiki-error">{error}</p>
          <button type="button" data-testid="wiki-retry" onClick={() => void load()}>
            Retry
          </button>
        </div>
      )}

      {!loading && !error && items != null && items.length === 0 && (
        <p data-testid="wiki-empty">
          No mastered concepts yet. Pages appear here once a concept reaches mastery.
        </p>
      )}

      {!loading && !error && items != null && items.length > 0 && visible.length === 0 && (
        <p data-testid="wiki-no-match">No mastered concepts match that search.</p>
      )}

      {!loading && !error && visible.length > 0 && (
        <ul data-testid="wiki-list" aria-label="Mastered concepts">
          {visible.map((item) => (
            <li key={item.id}>
              <button
                type="button"
                data-testid="wiki-row"
                data-concept-id={item.id}
                onClick={() => openWiki(item.id)}
              >
                {item.name}{' '}
                <span data-testid="wiki-confidence">{formatConfidence(item.confidence)}</span>
                {item.wiki_status === 'stale' && (
                  <span data-testid="wiki-stale-mark">May be outdated</span>
                )}
              </button>
            </li>
          ))}
        </ul>
      )}

      {!loading && !error && truncated && (
        <p data-testid="wiki-truncated-note">
          Showing the weakest {items?.length ?? 0}. Search the graph for more.
        </p>
      )}
    </div>
  );
}
