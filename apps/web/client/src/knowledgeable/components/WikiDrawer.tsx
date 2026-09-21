/**
 * Notebook reading pane (Phase 3, SPEC 7.2).
 *
 * Fixed overlay (`k-reader`): never pushes the chat, no scrim. Store-driven
 * by default (a single `KnowledgeableHost` mounted in `ChatRoute` renders
 * whatever `store/wikiDrawer.ts` holds); `conceptId`/`onClose` props take
 * over for controlled use and tests.
 */
import { useCallback, useEffect, useRef, useState } from 'react';
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
import { closeWiki, goBackWiki, openWiki, useOpenWikiConceptId, useWikiCanGoBack } from '../store/wikiDrawer';
import { confidenceWords, formatConfidence } from '../graphUtils';
import Button from './ui/Button';
import ConfidenceRing from './ui/ConfidenceRing';
import ErrorState from './ui/ErrorState';
import store from '~/store';

export interface WikiDrawerProps {
  conceptId?: string | null;
  onClose?: () => void;
}

/** Copy deck (SPEC 9): learner-facing reader errors. */
function errorMessage(err: unknown): string {
  if (err instanceof WikiNotReadyError) {
    return 'Notes appear once you have a good grip on this concept. Keep learning and check back.';
  }
  if (err instanceof WikiNotFoundError) {
    return 'Concept not found.';
  }
  return "Couldn't load this page. Try again.";
}

const TRY_MARKER = /^check-for-understanding\b/i;

/**
 * Split generated markdown into body + try-this section: a line beginning
 * "Check-for-understanding" (case-insensitive) starts the try section, with
 * the marker label stripped in favour of the "Try this" heading.
 */
export function splitTrySection(content: string): { main: string; tryText: string | null } {
  const lines = content.split('\n');
  const marker = lines.findIndex((line) => TRY_MARKER.test(line.trimStart()));
  if (marker === -1) {
    return { main: content, tryText: null };
  }
  const tryLines = lines.slice(marker);
  tryLines[0] = tryLines[0].replace(TRY_MARKER, '').replace(/^\s*[:—–-]\s*/, '').trimStart();
  const tryText = tryLines.join('\n').trim();
  return { main: lines.slice(0, marker).join('\n').trimEnd(), tryText: tryText || null };
}

export default function WikiDrawer({ conceptId, onClose }: WikiDrawerProps) {
  const storeId = useOpenWikiConceptId();
  const id = conceptId !== undefined ? conceptId : storeId;
  const handleClose = onClose ?? closeWiki;
  const canGoBack = useWikiCanGoBack();
  // Render parity with chat (F7): same LaTeX setting, same badge pipeline
  // fed with the page's live-derived annotations (validated, never stored —
  // the drawer is not a message).
  const LaTeXParsing = useRecoilValue(store.LaTeXParsing);
  const [page, setPage] = useState<WikiPage | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const closeRef = useRef<HTMLButtonElement>(null);
  const openerRef = useRef<Element | null>(null);
  const prevIdRef = useRef<string | null>(null);

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

  // Focus the close button whenever a new page opens (initial open and
  // chip navigation); restore the opener only when the drawer fully closes.
  useEffect(() => {
    if (id && prevIdRef.current !== id) {
      if (!prevIdRef.current) {
        openerRef.current = document.activeElement;
      }
      closeRef.current?.focus();
    } else if (!id && prevIdRef.current) {
      const opener = openerRef.current as HTMLElement | null;
      openerRef.current = null;
      opener?.focus?.();
    }
    prevIdRef.current = id;
  }, [id]);

  if (!id) {
    return null;
  }

  const confidence = page?.learner_confidence_at_generation ?? null;
  const { main, tryText } = splitTrySection(page?.personalized_content ?? '');
  const remarkPlugins = getRemarkPlugins(LaTeXParsing, sanitizeAnnotations(page?.concept_annotations));
  const rehypePlugins = getRehypePlugins();
  const components = getMarkdownComponents();

  return (
    <aside
      className="k-reader"
      role="dialog"
      aria-label={`Notebook page: ${page?.title ?? 'Notebook'}`}
      data-testid="wiki-drawer"
    >
      <button
        ref={closeRef}
        className="k-reader__close"
        type="button"
        aria-label="Close"
        data-testid="wiki-close"
        onClick={handleClose}
      >
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" aria-hidden="true"><path d="M6 6l12 12M18 6L6 18" /></svg>
      </button>
      {canGoBack && (
        <Button variant="ghost" onClick={() => goBackWiki()}>
          Back
        </Button>
      )}
      {loading && (
        <div role="status" data-testid="wiki-loading">
          <div className="k-skeleton k-skeleton--short" />
          <div className="k-skeleton" />
          <div className="k-skeleton" />
          <div className="k-skeleton" />
          <div className="k-skeleton" />
          <p>Writing your page. The first time can take a minute.</p>
        </div>
      )}
      {!loading && error && (
        <ErrorState
          message={error}
          onRetry={() => {
            const controller = new AbortController();
            load(id, controller.signal);
          }}
          retryLabel="Try again"
          testId="wiki-error"
          retryTestId="wiki-retry"
        />
      )}
      {!loading && !error && page && (
        <>
          <h3 className="k-reader__title" data-testid="wiki-title">
            {page.title}
          </h3>
          <div className="k-reader__meta" data-testid="wiki-confidence">
            <ConfidenceRing value={confidence} size={22} />
            {confidenceWords(confidence)}, {formatConfidence(confidence)}
          </div>
          {page.is_stale && (
            <p className="k-stale" data-testid="wiki-stale">
              May be outdated. It refreshes the next time you open it.
            </p>
          )}
          <p className="k-lede" data-testid="wiki-summary">
            {page.summary}
          </p>
          <div data-testid="wiki-content" className="markdown message-content k-read w-full break-words">
            <MarkdownBlocks
              content={main}
              remarkPlugins={remarkPlugins}
              rehypePlugins={rehypePlugins}
              components={components}
              animate={false}
              hydrated={false}
            />
          </div>
          {tryText && (
            <div className="k-try" data-testid="wiki-try">
              <strong>Try this</strong>
              <MarkdownBlocks
                content={tryText}
                remarkPlugins={remarkPlugins}
                rehypePlugins={rehypePlugins}
                components={components}
                animate={false}
                hydrated={false}
              />
            </div>
          )}
          {page.known_prerequisites.length > 0 && (
            <div>
              <p className="k-section-label">Builds on</p>
              <div className="k-chips" data-testid="wiki-prereqs">
                {page.known_prerequisites.map((prereq) => (
                  <button
                    key={prereq.concept_id}
                    type="button"
                    className="k-chip"
                    data-testid="wiki-prereq"
                    onClick={() => openWiki(prereq.concept_id)}
                  >
                    <ConfidenceRing value={prereq.learner_confidence} size={18} />
                    {prereq.name}
                  </button>
                ))}
              </div>
            </div>
          )}
          {page.related_concepts.length > 0 && (
            <div>
              <p className="k-section-label">Related</p>
              <div className="k-chips" data-testid="wiki-related">
                {page.related_concepts.map((related) => (
                  <button
                    key={related.concept_id}
                    type="button"
                    className="k-chip"
                    data-testid="wiki-related-item"
                    onClick={() => openWiki(related.concept_id)}
                  >
                    {related.name}
                  </button>
                ))}
              </div>
            </div>
          )}
        </>
      )}
    </aside>
  );
}
