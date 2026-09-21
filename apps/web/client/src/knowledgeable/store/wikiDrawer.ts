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

/** Previously open pages for the reader Back button; cleared by `closeWiki`. */
let history: string[] = [];

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
  if (openConceptId !== null) {
    history.push(openConceptId);
  }
  openConceptId = conceptId;
  emit();
}

/** Step back to the previously open page (chip navigation); `null` when at the start. */
export function goBackWiki(): string | null {
  const previous = history.pop() ?? null;
  if (previous === null) {
    return null;
  }
  openConceptId = previous;
  emit();
  return previous;
}

export function useWikiCanGoBack(): boolean {
  return useSyncExternalStore(subscribeWikiDrawer, () => history.length > 0, () => false);
}

export function closeWiki(): void {
  if (openConceptId === null && history.length === 0) {
    return;
  }
  openConceptId = null;
  history = [];
  emit();
}
