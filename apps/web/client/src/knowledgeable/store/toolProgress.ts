/**
 * Client-side store for M4 tool progress (T14, brick C1).
 *
 * The backend emits one `tool_progress` SSE frame per tool execution phase
 * (`started`/`finished`) in both wire modes. This module keeps
 * `Map<messageId, ToolActivity[]>` as a vanilla external store, mirroring
 * `store/annotations.ts`: no providers, no new dependencies.
 */
import { useSyncExternalStore } from 'react';

export type ToolPhase = 'started' | 'finished';

export interface ToolActivity {
  tool_name: string;
  call_id: string;
  phase: ToolPhase;
}

/** Bounds memory when a turn fans out many calls. */
export const MAX_STORED_ACTIVITIES = 20;

const EMPTY: ToolActivity[] = [];

let byMessage = new Map<string, ToolActivity[]>();
const listeners = new Set<() => void>();

function emit(): void {
  listeners.forEach((listener) => listener());
}

export function subscribeToolProgress(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** Referential-stable snapshot: stored array or shared empty. */
export function getToolProgress(messageId: string | undefined | null): ToolActivity[] {
  if (!messageId) {
    return EMPTY;
  }
  return byMessage.get(messageId) ?? EMPTY;
}

export function useToolProgress(messageId: string | undefined | null): ToolActivity[] {
  return useSyncExternalStore(
    subscribeToolProgress,
    () => getToolProgress(messageId),
    () => EMPTY,
  );
}

/**
 * Record one activity, merging by `call_id` so a `finished` frame upgrades
 * its `started` row instead of appending a duplicate.
 */
export function recordToolActivity(messageId: string, activity: ToolActivity): void {
  const current = byMessage.get(messageId) ?? [];
  const next = current.some((a) => a.call_id === activity.call_id)
    ? current.map((a) => (a.call_id === activity.call_id ? activity : a))
    : [...current, activity];
  byMessage.set(messageId, next.slice(-MAX_STORED_ACTIVITIES));
  emit();
}

export function clearToolProgress(messageId?: string): void {
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

function sanitizeActivity(value: unknown): ToolActivity | null {
  if (!isRecord(value)) {
    return null;
  }
  const { tool_name, call_id, phase } = value;
  if (typeof tool_name !== 'string' || tool_name.trim() === '') {
    return null;
  }
  if (typeof call_id !== 'string' || call_id.trim() === '') {
    return null;
  }
  if (phase !== 'started' && phase !== 'finished') {
    return null;
  }
  return { tool_name, call_id, phase };
}

/**
 * Validate + store one `tool_progress` SSE frame.
 *
 * Returns true when the envelope was consumed; false leaves the store
 * untouched. Never throws on malformed payloads.
 */
export function handleToolProgressEvent(data: unknown): boolean {
  if (!isRecord(data)) {
    return false;
  }
  const { messageId, tool_progress } = data;
  if (typeof messageId !== 'string' || messageId.trim() === '') {
    return false;
  }
  const activity = sanitizeActivity(tool_progress);
  if (!activity) {
    return false;
  }
  recordToolActivity(messageId, activity);
  return true;
}
