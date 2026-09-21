/**
 * Session map selection (SPEC 6.2).
 *
 * Module-level variable remembering the concept selected earlier this
 * session so the Map panel reloads it as its default state.
 */
let selectedConceptId: string | null = null;

export function getSelectedConceptId(): string | null {
  return selectedConceptId;
}

export function setSelectedConceptId(conceptId: string | null): void {
  selectedConceptId = conceptId;
}
