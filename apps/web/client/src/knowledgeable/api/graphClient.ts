import { clampDepth, clampLimit } from '../graphUtils';
import type { Neighborhood } from '../graphTypes';

export interface FetchNeighborhoodParams {
  conceptId: string;
  depth?: number;
  limit?: number;
  signal?: AbortSignal;
}

export class NeighborhoodValidationError extends Error {
  readonly code = 'validation_failed';
  constructor(message: string) {
    super(message);
    this.name = 'NeighborhoodValidationError';
  }
}

export class NeighborhoodNotFoundError extends Error {
  readonly code = 'not_found';
  constructor(message = 'concept not found') {
    super(message);
    this.name = 'NeighborhoodNotFoundError';
  }
}

export class NeighborhoodUnavailableError extends Error {
  readonly code = 'service_unavailable';
  constructor(message = 'graph service unavailable') {
    super(message);
    this.name = 'NeighborhoodUnavailableError';
  }
}

export class NeighborhoodError extends Error {
  readonly code: string;
  readonly status: number;
  constructor(message: string, status: number, code = 'unknown') {
    super(message);
    this.name = 'NeighborhoodError';
    this.status = status;
    this.code = code;
  }
}

/**
 * Build the request URL. Uses the `/api/*` adapter alias (Vite proxies `/api`
 * to Axum in dev); the backend serves the identical controller at canonical
 * `/v1/graph/neighborhood`.
 */export function buildNeighborhoodUrl(
  conceptId: string,
  depth?: number,
  limit?: number,
): string {
  const trimmed = conceptId.trim();
  if (!trimmed) {
    throw new NeighborhoodValidationError('concept_id is required');
  }
  const params = new URLSearchParams({
    concept_id: trimmed,
    depth: String(clampDepth(depth)),
    limit: String(clampLimit(limit)),
  });
  return `/api/graph/neighborhood?${params.toString()}`;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

/** Runtime shape guard: malformed payloads must throw, never crash render. */
export function parseNeighborhood(payload: unknown): Neighborhood {
  if (!isRecord(payload) || !Array.isArray(payload.nodes) || !Array.isArray(payload.edges)) {
    throw new NeighborhoodError('malformed neighborhood payload', 0, 'malformed_payload');
  }
  return payload as unknown as Neighborhood;
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
 * Fetch a bounded graph neighborhood for the visualizer.
 *
 * Maps the stable backend envelope (`code` + HTTP status) to typed errors:
 * - 400 -> `NeighborhoodValidationError`
 * - 404 -> `NeighborhoodNotFoundError`
 * - 503 -> `NeighborhoodUnavailableError`
 */
export async function fetchNeighborhood({
  conceptId,
  depth,
  limit,
  signal,
}: FetchNeighborhoodParams): Promise<Neighborhood> {
  const url = buildNeighborhoodUrl(conceptId, depth, limit);
  let response: Response;
  try {
    response = await fetch(url, { signal });
  } catch (err) {
    if (err instanceof DOMException && err.name === 'AbortError') {
      throw err;
    }
    throw new NeighborhoodUnavailableError('graph service unreachable');
  }
  if (response.ok) {
    return parseNeighborhood((await response.json()) as unknown);
  }
  const envelope = await readEnvelope(response);
  const message =
    typeof envelope.message === 'string' && envelope.message
      ? envelope.message
      : `graph request failed (${response.status})`;
  if (response.status === 400) {
    throw new NeighborhoodValidationError(message);
  }
  if (response.status === 404) {
    throw new NeighborhoodNotFoundError(message);
  }
  if (response.status === 503) {
    throw new NeighborhoodUnavailableError(message);
  }
  const code = typeof envelope.code === 'string' ? envelope.code : 'unknown';
  throw new NeighborhoodError(message, response.status, code);
}

export interface ConceptSearchHit {
  id: string;
  canonical_name: string;
  canonical_statement: string;
}

function isSearchHit(value: unknown): value is ConceptSearchHit {
  if (typeof value !== 'object' || value === null) {
    return false;
  }
  const hit = value as Record<string, unknown>;
  return (
    typeof hit.id === 'string' &&
    hit.id.trim() !== '' &&
    typeof hit.canonical_name === 'string' &&
    hit.canonical_name.trim() !== '' &&
    typeof hit.canonical_statement === 'string'
  );
}

/**
 * Search concepts by name/statement for pickers. Empty queries short-circuit
 * to `[]` without fetching (typeahead-friendly).
 */
export async function searchConcepts(
  query: string,
  options?: { limit?: number; signal?: AbortSignal },
): Promise<ConceptSearchHit[]> {
  const trimmed = query.trim();
  if (!trimmed) {
    return [];
  }
  const params = new URLSearchParams({ q: trimmed, limit: String(clampLimit(options?.limit)) });
  let response: Response;
  try {
    response = await fetch(`/api/concepts/search?${params.toString()}`, {
      signal: options?.signal,
    });
  } catch (err) {
    if (err instanceof DOMException && err.name === 'AbortError') {
      throw err;
    }
    throw new NeighborhoodUnavailableError('concept search unreachable');
  }
  if (!response.ok) {
    const envelope = await readEnvelope(response);
    const message =
      typeof envelope.message === 'string' && envelope.message
        ? envelope.message
        : `concept search failed (${response.status})`;
    if (response.status === 503) {
      throw new NeighborhoodUnavailableError(message);
    }
    throw new NeighborhoodError(message, response.status);
  }
  const payload: unknown = await response.json();
  if (!Array.isArray(payload)) {
    throw new NeighborhoodError('malformed search payload', 0, 'malformed_payload');
  }
  return payload.filter(isSearchHit);
}
