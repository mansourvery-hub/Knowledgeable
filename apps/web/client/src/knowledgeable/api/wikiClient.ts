/**
 * Typed client for `GET /api/concepts/:id/wiki` (M8, brick G4).
 *
 * Mirrors `api/graphClient.ts`: stable backend envelope mapped to typed
 * errors, malformed payloads rejected instead of crashing render. Viewing
 * loads from the SQLite cache — this client never triggers generation
 * itself; the server generates lazily on miss.
 */

export interface WikiPrerequisite {
  concept_id: string;
  name: string;
  learner_confidence: number;
}

export interface WikiRelated {
  concept_id: string;
  name: string;
  relation_type: string;
}

export interface WikiPage {
  id: string;
  learner_id: string;
  concept_id: string;
  title: string;
  summary: string;
  personalized_content: string;
  known_prerequisites: WikiPrerequisite[];
  related_concepts: WikiRelated[];
  learner_confidence_at_generation: number;
  version: number;
  is_stale: boolean;
  created_at: string;
  updated_at: string;
}

export class WikiValidationError extends Error {
  readonly code = 'validation_failed';
  constructor(message: string) {
    super(message);
    this.name = 'WikiValidationError';
  }
}

export class WikiNotFoundError extends Error {
  readonly code = 'not_found';
  constructor(message = 'concept not found') {
    super(message);
    this.name = 'WikiNotFoundError';
  }
}

export class WikiNotReadyError extends Error {
  readonly code = 'wiki_not_ready';
  constructor(message = 'concept below wiki mastery threshold') {
    super(message);
    this.name = 'WikiNotReadyError';
  }
}

export class WikiUnavailableError extends Error {
  readonly code = 'service_unavailable';
  constructor(message = 'wiki service unavailable') {
    super(message);
    this.name = 'WikiUnavailableError';
  }
}

export class WikiError extends Error {
  readonly code: string;
  readonly status: number;
  constructor(message: string, status: number, code = 'unknown') {
    super(message);
    this.name = 'WikiError';
    this.status = status;
    this.code = code;
  }
}

/**
 * Mastered-concept row for the wiki browser (Phase 5a, W1).
 *
 * Mirrors `GET /api/concepts/mastered`: weakest-first server order,
 * `wiki_status` is `ready` (fresh page), `stale` (flagged by graph churn),
 * or `none` (no page yet — the drawer generates on miss).
 */
export interface MasteredConceptItem {
  id: string;
  name: string;
  confidence: number;
  wiki_status: 'ready' | 'stale' | 'none';
}

export interface MasteredList {
  items: MasteredConceptItem[];
  truncated: boolean;
}

/** Backend caps the list here; the client filters names locally. */
export const MASTERED_LIST_LIMIT = 200;

export function buildWikiUrl(conceptId: string): string {
  const trimmed = conceptId.trim();
  if (!trimmed) {
    throw new WikiValidationError('concept id is required');
  }
  return `/api/concepts/${encodeURIComponent(trimmed)}/wiki`;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

/** Runtime shape guard: malformed payloads must throw, never crash render. */
export function parseWikiPage(payload: unknown): WikiPage {
  if (
    !isRecord(payload) ||
    typeof payload.id !== 'string' ||
    typeof payload.title !== 'string' ||
    typeof payload.summary !== 'string' ||
    typeof payload.personalized_content !== 'string' ||
    !Array.isArray(payload.known_prerequisites) ||
    !Array.isArray(payload.related_concepts)
  ) {
    throw new WikiError('malformed wiki payload', 0, 'malformed_payload');
  }
  return payload as unknown as WikiPage;
}

interface ErrorEnvelope {
  code?: unknown;
  message?: unknown;
}

async function readEnvelope(response: Response): Promise<ErrorEnvelope> {
  try {
    const data: unknown = await response.json();
    return isRecord(data) ? data : {};
  } catch {
    return {};
  }
}

/**
 * Fetch the cached wiki page for a concept.
 *
 * - 400 -> `WikiValidationError`
 * - 404 + `wiki_not_ready` -> `WikiNotReadyError` (keep learning)
 * - 404 otherwise -> `WikiNotFoundError`
 * - 503 -> `WikiUnavailableError`
 */
export async function fetchWikiPage(
  conceptId: string,
  options?: { signal?: AbortSignal },
): Promise<WikiPage> {
  const url = buildWikiUrl(conceptId);
  let response: Response;
  try {
    response = await fetch(url, { signal: options?.signal });
  } catch (err) {
    if (err instanceof DOMException && err.name === 'AbortError') {
      throw err;
    }
    throw new WikiUnavailableError('wiki service unreachable');
  }
  if (response.ok) {
    return parseWikiPage((await response.json()) as unknown);
  }
  const envelope = await readEnvelope(response);
  const message =
    typeof envelope.message === 'string' && envelope.message
      ? envelope.message
      : `wiki request failed (${response.status})`;
  if (response.status === 400) {
    throw new WikiValidationError(message);
  }
  if (response.status === 404) {
    if (envelope.code === 'wiki_not_ready') {
      throw new WikiNotReadyError(message);
    }
    throw new WikiNotFoundError(message);
  }
  if (response.status === 503) {
    throw new WikiUnavailableError(message);
  }
  const code = typeof envelope.code === 'string' ? envelope.code : 'unknown';
  throw new WikiError(message, response.status, code);
}

function isMasteredItem(value: unknown): value is MasteredConceptItem {
  if (!isRecord(value)) {
    return false;
  }
  return (
    typeof value.id === 'string' &&
    typeof value.name === 'string' &&
    typeof value.confidence === 'number' &&
    (value.wiki_status === 'ready' || value.wiki_status === 'stale' || value.wiki_status === 'none')
  );
}

/** Runtime shape guard: malformed payloads must throw, never crash render. */
export function parseMasteredList(payload: unknown): MasteredList {
  if (!isRecord(payload) || !Array.isArray(payload.items)) {
    throw new WikiError('malformed mastered list payload', 0, 'malformed_payload');
  }
  if (!payload.items.every(isMasteredItem)) {
    throw new WikiError('malformed mastered list item', 0, 'malformed_payload');
  }
  return {
    items: payload.items,
    truncated: payload.truncated === true,
  };
}

/**
 * Fetch the bounded mastered-concepts list for the wiki browser.
 *
 * Maps the stable backend envelope to typed errors:
 * - 400 -> `WikiValidationError`
 * - 503 -> `WikiUnavailableError`
 */
export async function fetchMasteredConcepts(
  options?: { limit?: number; signal?: AbortSignal },
): Promise<MasteredList> {
  const params = new URLSearchParams({
    limit: String(options?.limit ?? MASTERED_LIST_LIMIT),
  });
  const url = `/api/concepts/mastered?${params.toString()}`;
  let response: Response;
  try {
    response = await fetch(url, { signal: options?.signal });
  } catch (err) {
    if (err instanceof DOMException && err.name === 'AbortError') {
      throw err;
    }
    throw new WikiUnavailableError('wiki service unreachable');
  }
  if (response.ok) {
    return parseMasteredList((await response.json()) as unknown);
  }
  const envelope = await readEnvelope(response);
  const message =
    typeof envelope.message === 'string' && envelope.message
      ? envelope.message
      : `wiki request failed (${response.status})`;
  if (response.status === 400) {
    throw new WikiValidationError(message);
  }
  if (response.status === 503) {
    throw new WikiUnavailableError(message);
  }
  const code = typeof envelope.code === 'string' ? envelope.code : 'unknown';
  throw new WikiError(message, response.status, code);
}
