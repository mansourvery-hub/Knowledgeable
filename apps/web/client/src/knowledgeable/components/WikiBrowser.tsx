import { useCallback, useEffect, useRef, useState } from 'react';
import { useRecoilValue } from 'recoil';
import { BookOpen } from 'lucide-react';
import { openWiki, useOpenWikiConceptId } from '../store/wikiDrawer';
import {
  fetchMasteredConcepts,
  WikiUnavailableError,
  WikiValidationError,
  type MasteredConceptItem,
} from '../api/wikiClient';
import { formatConfidence } from '../graphUtils';
import store from '~/store';

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
  // F7/F8 product call: row percentages render only with the debug toggle.
  const showConfidence = useRecoilValue(store.showConfidenceDebug);
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
          {visible.map((item) => {
            const isOpen = openConceptId === item.id;
            return (
              <li key={item.id}>
                <div
                  role="button"
                  tabIndex={0}
                  data-testid="wiki-row"
                  data-concept-id={item.id}
                  onClick={() => openWiki(item.id)}
                  onKeyDown={(event) => {
                    if (event.key === 'Enter' || event.key === ' ') {
                      event.preventDefault();
                      openWiki(item.id);
                    }
                  }}
                  className={`group relative flex h-12 w-full items-center rounded-lg outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-text-primary ${
                    isOpen
                      ? 'bg-surface-active-alt before:absolute before:bottom-1 before:left-0 before:top-1 before:w-0.5 before:rounded-full before:bg-text-primary'
                      : 'hover:bg-surface-active-alt'
                  }`}
                >
                  <div className="flex w-full min-w-0 grow cursor-pointer items-center gap-2 overflow-hidden rounded-lg px-2 text-left">
                    <BookOpen className="icon-sm shrink-0 text-text-secondary" aria-hidden="true" />
                    <span className="min-w-0 flex-1 truncate" title={item.name}>
                      {item.name}
                    </span>
                    {item.wiki_status === 'stale' && (
                      <span
                        data-testid="wiki-stale-mark"
                        className="shrink-0 text-xs text-text-secondary"
                      >
                        May be outdated
                      </span>
                    )}
                    {showConfidence && (
                      <span
                        data-testid="wiki-confidence"
                        className="shrink-0 text-xs text-text-secondary"
                      >
                        {formatConfidence(item.confidence)}
                      </span>
                    )}
                  </div>
                </div>
              </li>
            );
          })}
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
