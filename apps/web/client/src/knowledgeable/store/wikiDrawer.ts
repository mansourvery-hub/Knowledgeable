/**
 * Wiki drawer selection store (M8, brick G4).
 *
 * Holds the currently open wiki concept (if any) as vanilla external state,
 * mirroring the annotations/progress stores: concept badges anywhere in the
 * chat tree can open the drawer without prop drilling, and a single host
 * mounted in `ChatRoute` renders it.
 */
import { useSyncExternalStore } from 'react';

let openConceptId: string | null = null;
const listeners = new Set<() => void>();

function emit(): void {
  listeners.forEach((listener) => listener());
}

export function subscribeWikiDrawer(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function getOpenWikiConceptId(): string | null {
  return openConceptId;
}

export function useOpenWikiConceptId(): string | null {
  return useSyncExternalStore(subscribeWikiDrawer, getOpenWikiConceptId, () => null);
}

export function openWiki(conceptId: string): void {
  if (typeof conceptId !== 'string' || conceptId.trim() === '') {
    return;
  }
  if (openConceptId === conceptId) {
    return;
  }
  openConceptId = conceptId;
  emit();
}

export function closeWiki(): void {
  if (openConceptId === null) {
    return;
  }
  openConceptId = null;
  emit();
}
