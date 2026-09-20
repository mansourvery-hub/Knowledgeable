/**
 * Personal Knowledge Wiki slide-over drawer (M8, brick G4).
 *
 * Store-driven by default (single host mounted in `ChatRoute` renders
 * whatever `store/wikiDrawer.ts` holds); `conceptId`/`onClose` props take
 * over for controlled use and tests. Viewing loads from the SQLite cache —
 * generation happens server-side on miss, never from this component.
 */
import { useCallback, useEffect, useState } from 'react';
import { useRecoilValue } from 'recoil';
import MarkdownBlocks from '~/components/Chat/Messages/Content/MarkdownBlocks';
import { getMarkdownComponents, getRehypePlugins, getRemarkPlugins } from '~/components/Chat/Messages/Content/markdownConfig';
import {
  WikiNotFoundError,
  WikiNotReadyError,
  fetchWikiPage,
  type WikiPage,
} from '../api/wikiClient';
import { sanitizeAnnotations } from '../store/annotations';
import { closeWiki, useOpenWikiConceptId } from '../store/wikiDrawer';
import store from '~/store';

export interface WikiDrawerProps {
  conceptId?: string | null;
  onClose?: () => void;
}

function errorMessage(err: unknown): string {
  if (err instanceof WikiNotReadyError) {
    return "This concept isn't ready for a wiki page yet — keep learning and check back.";
  }
  if (err instanceof WikiNotFoundError) {
    return 'Concept not found.';
  }
  return "Couldn't load the wiki page.";
}

export default function WikiDrawer({ conceptId, onClose }: WikiDrawerProps) {
  const storeId = useOpenWikiConceptId();
  const id = conceptId !== undefined ? conceptId : storeId;
  const handleClose = onClose ?? closeWiki;
  // Render parity with chat (F7): same LaTeX setting, same badge pipeline
  // fed with the page's live-derived annotations (validated, never stored —
  // the drawer is not a message). Percentages follow the debug toggle.
  const LaTeXParsing = useRecoilValue(store.LaTeXParsing);
  const showConfidence = useRecoilValue(store.showConfidenceDebug);
  const [page, setPage] = useState<WikiPage | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(
    (target: string, signal: AbortSignal) => {
      setLoading(true);
      setError(null);
      fetchWikiPage(target, { signal }).then(
        (result) => {
          if (!signal.aborted) {
            setPage(result);
            setLoading(false);
          }
        },
        (err: unknown) => {
          if (signal.aborted || (err instanceof DOMException && err.name === 'AbortError')) {
            return;
          }
          setPage(null);
          setError(errorMessage(err));
          setLoading(false);
        },
      );
    },
    [],
  );

  useEffect(() => {
    setPage(null);
    setError(null);
    if (!id) {
      setLoading(false);
      return;
    }
    const controller = new AbortController();
    load(id, controller.signal);
    return () => controller.abort();
  }, [id, load]);

  useEffect(() => {
    if (!id) {
      return;
    }
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        handleClose();
      }
    };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [id, handleClose]);

  if (!id) {
    return null;
  }

  return (
    <aside
      data-testid="wiki-drawer"
      role="dialog"
      aria-label="Personal knowledge wiki"
      className="flex h-full w-80 flex-col border-l border-border-light bg-surface-primary"
    >
      <div className="flex items-center justify-between p-3">
        <h2 data-testid="wiki-heading">Personal Wiki</h2>
        <button type="button" data-testid="wiki-close" onClick={handleClose} aria-label="Close wiki">
          ×
        </button>
      </div>
      {loading && (
        <div role="status" data-testid="wiki-loading">
          Loading wiki page… (first view generates your personal page and can take a minute)
        </div>
      )}
      {error && !loading && (
        <div role="alert" data-testid="wiki-error">
          {error}
          <button
            type="button"
            data-testid="wiki-retry"
            onClick={() => {
              const controller = new AbortController();
              load(id, controller.signal);
            }}
          >
            Retry
          </button>
        </div>
      )}
      {page && !loading && !error && (
        <div className="overflow-y-auto p-3">
          <h3 data-testid="wiki-title">{page.title}</h3>
          {page.is_stale && (
            <p data-testid="wiki-stale">May be outdated — refreshes on next view.</p>
          )}
          {showConfidence && (
            <div data-testid="wiki-confidence">
              Confidence {Math.round(page.learner_confidence_at_generation * 100)}%
              <div
                aria-hidden="true"
                data-testid="wiki-confidence-bar"
                style={{ width: `${Math.round(page.learner_confidence_at_generation * 100)}%` }}
              />
            </div>
          )}
          <p data-testid="wiki-summary">{page.summary}</p>
          {/* Same content-typography container as chat assistant messages
            (MessageContent): identical type scale, spacing, and dark-mode
            inversion for the same rendered elements. */}
          <div
            data-testid="wiki-content"
            className="markdown prose message-content dark:prose-invert light w-full break-words"
          >
            <MarkdownBlocks
              content={page.personalized_content}
              remarkPlugins={getRemarkPlugins(
                LaTeXParsing,
                sanitizeAnnotations(page.concept_annotations),
              )}
              rehypePlugins={getRehypePlugins()}
              components={getMarkdownComponents()}
              animate={false}
              hydrated={false}
            />
          </div>
          {page.known_prerequisites.length > 0 && (
            <ul data-testid="wiki-prereqs" aria-label="Prerequisites">
              {page.known_prerequisites.map((prereq) => (
                <li key={prereq.concept_id} data-testid="wiki-prereq">
                  {prereq.name}
                  {showConfidence && <> · {Math.round(prereq.learner_confidence * 100)}%</>}
                </li>
              ))}
            </ul>
          )}
          {page.related_concepts.length > 0 && (
            <ul data-testid="wiki-related" aria-label="Related concepts">
              {page.related_concepts.map((related) => (
                <li key={related.concept_id} data-testid="wiki-related-item">
                  {related.name}
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </aside>
  );
}
