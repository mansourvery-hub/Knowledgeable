/**
 * remark plugin for M7 concept highlighting (T11c).
 *
 * Rewrites narrative `text` nodes that mention annotated concepts into
 * `concept-highlight` nodes (`hName`/`hProperties` consumed by
 * `components/ConceptHighlight.tsx` via `getMarkdownComponents`).
 *
 * Safety rules (M7 exit gate):
 * - Only `text` nodes are visited. Fenced code (`code`), inline code
 *   (`inlineCode`), math (`math`/`inlineMath`), and raw HTML (`html`) carry
 *   their content in `value`, never in `text` children, so they pass through
 *   byte-identical.
 * - Whole-word, case-insensitive matching with longest-name-wins, mirroring
 *   `domain::match_mentions` on the backend.
 * - No annotations (streaming in progress) → tree untouched.
 */
import { visit } from 'unist-util-visit';
import type { Node } from 'unist';
import type { ConceptAnnotation } from '../types';

export const CONCEPT_HIGHLIGHT_NODE = 'concept-highlight';

export interface ConceptHighlightOptions {
  annotations?: ConceptAnnotation[];
}

interface TextNode extends Node {
  type: 'text';
  value: string;
}

interface HighlightNode extends Node {
  type: string;
  data?: {
    hName?: string;
    hProperties?: Record<string, unknown>;
  };
  children?: TextNode[];
}

interface ParentNode extends Node {
  children?: Array<TextNode | HighlightNode | Node>;
}

function isWordChar(char: string): boolean {
  return /[\p{L}\p{N}_]/u.test(char);
}

interface Span {
  start: number;
  end: number;
  annotation: ConceptAnnotation;
}

/** All whole-word occurrences of every annotation name in `lower`. */
function findSpans(
  lower: string,
  annotations: ConceptAnnotation[],
): Span[] {
  const spans: Span[] = [];
  const ordered = [...annotations].sort((a, b) => b.name.length - a.name.length);
  const claimed: Array<[number, number]> = [];
  for (const annotation of ordered) {
    const needle = annotation.name.toLowerCase();
    if (needle.trim().length < 2) {
      continue;
    }
    let from = 0;
    for (;;) {
      const at = lower.indexOf(needle, from);
      if (at < 0) {
        break;
      }
      from = at + 1;
      const end = at + needle.length;
      const leftOk = at === 0 || !isWordChar(lower[at - 1]);
      const rightOk = end >= lower.length || !isWordChar(lower[end]);
      const overlaps = claimed.some(([s, e]) => at < e && s < end);
      if (leftOk && rightOk && !overlaps) {
        spans.push({ start: at, end, annotation });
        claimed.push([at, end]);
      }
    }
  }
  spans.sort((a, b) => a.start - b.start);
  return spans;
}

export function remarkConceptHighlight(options?: ConceptHighlightOptions) {
  // Product decision (T25): chat surfaces KNOWN concepts only. Weak/new
  // mentions render as plain text — learner health stays visible in the
  // graph explorer, and chat stays free of percentages and review noise.
  const annotations = (options?.annotations ?? []).filter(
    (a) =>
      a &&
      typeof a.name === 'string' &&
      a.name.trim().length >= 2 &&
      a.status === 'known',
  );
  return (tree: Node) => {
    if (annotations.length === 0) {
      return;
    }
    visit(tree, 'text', (node: TextNode, index?: number, parent?: ParentNode) => {
      if (index == null || !parent?.children) {
        return;
      }
      const value = node.value;
      if (!value) {
        return;
      }
      const spans = findSpans(value.toLowerCase(), annotations);
      if (spans.length === 0) {
        return;
      }
      const segments: Array<TextNode | HighlightNode> = [];
      let cursor = 0;
      for (const span of spans) {
        if (span.start > cursor) {
          segments.push({ type: 'text', value: value.slice(cursor, span.start) });
        }
        const { annotation } = span;
        segments.push({
          type: CONCEPT_HIGHLIGHT_NODE,
          data: {
            hName: CONCEPT_HIGHLIGHT_NODE,
            hProperties: {
              conceptId: annotation.concept_id,
              name: annotation.name,
              status: annotation.status,
              confidence: annotation.learner_confidence,
            },
          },
          children: [{ type: 'text', value: value.slice(span.start, span.end) }],
        });
        cursor = span.end;
      }
      if (cursor < value.length) {
        segments.push({ type: 'text', value: value.slice(cursor) });
      }
      parent.children.splice(index, 1, ...segments);
      return index + segments.length;
    });
  };
}
