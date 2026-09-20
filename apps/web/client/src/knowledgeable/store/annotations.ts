/**
 * Client-side store for M7 concept annotations (T11a).
 *
 * The backend emits one `concept_annotations` SSE frame per completed tutor
 * turn; this module keeps `Map<messageId, ConceptAnnotation[]>` outside React
 * so the remark plugin, message renderers, and (later) the graph explorer can
 * share it without new providers or dependencies. Vanilla external store +
 * `useSyncExternalStore`, mirroring the isolation of the other knowledgeable
 * modules.
 */
import { useEffect } from 'react';
import { useSyncExternalStore } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { QueryKeys } from 'librechat-data-provider';
import type { TMessage } from 'librechat-data-provider';
import type { ConceptAnnotation, ConceptAnnotationStatus } from '../types';

/** Mirrors `domain::MAX_ANNOTATIONS_PER_TURN`. */
export const MAX_STORED_PER_MESSAGE = 20;

const STATUSES: ReadonlySet<string> = new Set(['known', 'weak', 'new']);
const EMPTY: ConceptAnnotation[] = [];

let byMessage = new Map<string, ConceptAnnotation[]>();
const listeners = new Set<() => void>();

function emit(): void {
  listeners.forEach((listener) => listener());
}

export function subscribeMessageAnnotations(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** Referential-stable snapshot: stored array or shared empty. */
export function getMessageAnnotations(
  messageId: string | undefined | null,
): ConceptAnnotation[] {
  if (!messageId) {
    return EMPTY;
  }
  return byMessage.get(messageId) ?? EMPTY;
}

export function useMessageAnnotations(
  messageId: string | undefined | null,
): ConceptAnnotation[] {
  useHydrateHistoryAnnotations(messageId);
  return useSyncExternalStore(
    subscribeMessageAnnotations,
    () => getMessageAnnotations(messageId),
    () => EMPTY,
  );
}

/** History payloads may carry live-derived annotations (F7 Brick 1). */
type HistoryMessage = Pick<TMessage, 'messageId'> & {
  concept_annotations?: unknown;
};

/**
 * History hydration (F7 Brick 2): message history arrives through the
 * upstream messages query, which this boundary must not rewire. Instead this
 * effect adopts each rendered message's embedded `concept_annotations`
 * through the same validated path as live SSE frames. Runs per message,
 * idempotent, and never throws into render: live frames always win because
 * the SSE dispatcher writes directly and this skips stored ids.
 */
export function useHydrateHistoryAnnotations(
  messageId: string | undefined | null,
): void {
  const queryClient = useQueryClient();
  useEffect(() => {
    if (!messageId || byMessage.has(messageId)) {
      return;
    }
    const cached = queryClient.getQueriesData<TMessage[]>({
      queryKey: [QueryKeys.messages],
    });
    for (const [, data] of cached) {
      if (!Array.isArray(data)) {
        continue;
      }
      const match = (data as HistoryMessage[]).find((m) => m?.messageId === messageId);
      const embedded = match?.concept_annotations;
      if (Array.isArray(embedded) && embedded.length > 0) {
        handleConceptAnnotationsEvent({ messageId, concept_annotations: embedded });
        return;
      }
    }
  }, [queryClient, messageId]);
}

export function setMessageAnnotations(
  messageId: string,
  annotations: ConceptAnnotation[],
): void {
  byMessage.set(messageId, annotations.slice(0, MAX_STORED_PER_MESSAGE));
  emit();
}

export function clearMessageAnnotations(messageId?: string): void {
  if (messageId == null) {
    if (byMessage.size === 0) {
      return;
    }
    byMessage = new Map();
  } else {
    if (!byMessage.delete(messageId)) {
      return;
    }
  }
  emit();
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function sanitizeAnnotation(value: unknown): ConceptAnnotation | null {
  if (!isRecord(value)) {
    return null;
  }
  const { concept_id, name, learner_confidence, status } = value;
  if (typeof concept_id !== 'string' || concept_id.trim() === '') {
    return null;
  }
  if (typeof name !== 'string' || name.trim() === '') {
    return null;
  }
  if (typeof status !== 'string' || !STATUSES.has(status)) {
    return null;
  }
  let confidence: number | null = null;
  if (typeof learner_confidence === 'number' && !Number.isNaN(learner_confidence)) {
    confidence = learner_confidence;
  }
  return {
    concept_id,
    name,
    learner_confidence: confidence,
    status: status as ConceptAnnotationStatus,
  };
}

/**
 * Validate + store one `concept_annotations` SSE frame.
 *
 * Returns true when the envelope was well-formed and consumed (even when it
 * carries zero valid entries); false leaves the store untouched. Never throws
 * on malformed payloads — a broken frame must not break the stream.
 */
export function handleConceptAnnotationsEvent(data: unknown): boolean {
  if (!isRecord(data)) {
    return false;
  }
  const { messageId, concept_annotations } = data;
  if (typeof messageId !== 'string' || messageId.trim() === '') {
    return false;
  }
  if (!Array.isArray(concept_annotations)) {
    return false;
  }
  const clean: ConceptAnnotation[] = [];
  for (const entry of concept_annotations) {
    const annotation = sanitizeAnnotation(entry);
    if (annotation && clean.length < MAX_STORED_PER_MESSAGE) {
      clean.push(annotation);
    }
  }
  setMessageAnnotations(messageId, clean);
  return true;
}
