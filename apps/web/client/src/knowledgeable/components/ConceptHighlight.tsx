/**
 * Rendered concept badge for M7 chat highlighting (T11d).
 *
 * Receives its data from `remarkConceptHighlight` via `hProperties`
 * (registered as `concept-highlight` in `getMarkdownComponents`):
 * - known (>= 0.80): dotted accent underline, style-only.
 * - weak (< 0.80): marker wash, style-only.
 * - new: accent wash, style-only.
 *
 * F17 product call: badges never pop up — no tooltip on hover or focus.
 * Status (and debug-gated confidence) lives in the accessible label;
 * click/Enter/Space opens the wiki drawer (M8). Missing/invalid props
 * degrade to plain text so a malformed frame never breaks render.
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
  // default. Status (and the percentage, with the debug toggle on) lives
  // in the accessible label; F17 removed the visual popup entirely.
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
      className={`relative ${STATUS_CLASS[status]}${clickable ? ' cursor-pointer' : ''}`}
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
    </span>
  );
}
