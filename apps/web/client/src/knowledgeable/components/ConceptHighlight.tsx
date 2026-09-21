/**
 * Rendered concept badge for M7 chat highlighting (T11d).
 *
 * Receives its data from `remarkConceptHighlight` via `hProperties`
 * (registered as `concept-highlight` in `getMarkdownComponents`):
 * - known (>= 0.80): dotted accent underline + tooltip.
 * - weak (< 0.80): marker wash + prerequisite tooltip.
 * - new: accent wash + new-concept tooltip.
 *
 * Identity comes from `k-concept`/`k-tip` (Phase 4); structure, data-testids
 * and hover/focus behaviour are unchanged. Missing/invalid props degrade to
 * plain text so a malformed frame never breaks render.
 * Wiki-drawer click wiring lands with M8.
 */
import type { ReactNode } from 'react';
import { useRecoilValue } from 'recoil';
import type { ConceptAnnotationStatus } from '../types';
import { formatConfidence } from '../graphUtils';
import { openWiki } from '../store/wikiDrawer';
import store from '~/store';

export interface ConceptHighlightProps {
  conceptId?: string;
  name?: string;
  status?: ConceptAnnotationStatus;
  confidence?: number | null;
  children?: ReactNode;
}

const STATUS_LABEL: Record<ConceptAnnotationStatus, string> = {
  known: 'Known concept',
  weak: 'Needs review',
  new: 'New concept',
};

const STATUS_CLASS: Record<ConceptAnnotationStatus, string> = {
  known: 'k-concept k-concept--known',
  weak: 'k-concept k-concept--weak',
  new: 'k-concept k-concept--new',
};

function isStatus(value: unknown): value is ConceptAnnotationStatus {
  return value === 'known' || value === 'weak' || value === 'new';
}

export default function ConceptHighlight({
  conceptId,
  name,
  status,
  confidence,
  children,
}: ConceptHighlightProps) {
  if (!isStatus(status)) {
    return <>{children}</>;
  }
  // F7/F8 product call: confidence percentages are a debug aid, off by
  // default. The tooltip shows title + status; the percentage renders (and
  // is announced) only with the debug toggle on.
  const showConfidence = useRecoilValue(store.showConfidenceDebug);
  const label = typeof name === 'string' && name ? name : 'Concept';
  const confidenceText = formatConfidence(
    typeof confidence === 'number' ? confidence : null,
  );
  const clickable = typeof conceptId === 'string' && conceptId !== '';
  const accessibleLabel = showConfidence
    ? `${label}, ${STATUS_LABEL[status]}, confidence ${confidenceText}${
        clickable ? ', open wiki' : ''
      }`
    : `${label}, ${STATUS_LABEL[status]}${clickable ? ', open wiki' : ''}`;
  return (
    <span
      data-testid="concept-highlight"
      data-status={status}
      data-concept-id={conceptId ?? ''}
      tabIndex={0}
      role={clickable ? 'button' : undefined}
      aria-label={accessibleLabel}
      className={`group relative ${STATUS_CLASS[status]}${clickable ? ' cursor-pointer' : ''}`}
      onClick={clickable ? () => openWiki(conceptId as string) : undefined}
      onKeyDown={
        clickable
          ? (event) => {
              if (event.key === 'Enter' || event.key === ' ') {
                event.preventDefault();
                openWiki(conceptId as string);
              }
            }
          : undefined
      }
    >
      {children ?? label}
      <span
        data-testid="concept-highlight-tooltip"
        role="tooltip"
        className="k-tip invisible absolute bottom-full left-0 z-10 mb-1 w-max max-w-60 group-hover:visible group-focus-within:visible"
      >
        <strong>{label}</strong>
        {showConfidence && (
          <span data-testid="concept-highlight-confidence"> · {confidenceText}</span>
        )}
        <br />
        <span>{STATUS_LABEL[status]}</span>
      </span>
    </span>
  );
}
