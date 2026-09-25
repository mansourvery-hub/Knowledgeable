/**
 * Tester API token for bearer-gated deployments (`PUBLIC_API_TOKEN`).
 *
 * Tester-only surface (no product commitment): the token lives in
 * localStorage, is captured once from `?ktoken=` (then stripped from the
 * URL), is editable behind the `?kdebug` settings entry, and is attached to
 * every axios call plus both SSE submission paths. The stored token always
 * wins over the session token (which is a `local-session-token` dummy while
 * auth is off). With no stored token nothing changes anywhere.
 *
 * Never logged, never rendered back: the settings field is a password input
 * and only reports set/unset status.
 */
import axios from 'axios';
import type { AxiosRequestHeaders } from 'axios';

export const API_TOKEN_KEY = 'knowledgeable.apiToken';
export const API_TOKEN_PARAM = 'ktoken';

function readParams(search?: string): URLSearchParams {
  if (typeof search === 'string') {
    return new URLSearchParams(search);
  }
  if (typeof window !== 'undefined') {
    return new URLSearchParams(window.location.search);
  }
  return new URLSearchParams();
}

function readStored(): string {
  if (typeof window === 'undefined') {
    return '';
  }
  try {
    return (window.localStorage.getItem(API_TOKEN_KEY) ?? '').trim();
  } catch {
    return '';
  }
}

/** Current token: `?ktoken=` capture wins once (persisted + stripped), else stored. */
export function getApiToken(search?: string): string {
  const fromParam = (readParams(search).get(API_TOKEN_PARAM) ?? '').trim();
  if (fromParam === '' || typeof window === 'undefined') {
    return readStored();
  }
  try {
    window.localStorage.setItem(API_TOKEN_KEY, fromParam);
  } catch {
    // Private mode etc: fall through to the in-memory value for this load.
  }
  try {
    const url = new URL(window.location.href);
    url.searchParams.delete(API_TOKEN_PARAM);
    window.history.replaceState(null, '', url.toString());
  } catch {
    // Non-http(s) contexts: harmless, the token still applies this load.
  }
  return fromParam;
}

export function setApiToken(token: string): void {
  if (typeof window === 'undefined') {
    return;
  }
  const clean = token.trim();
  try {
    if (clean === '') {
      window.localStorage.removeItem(API_TOKEN_KEY);
    } else {
      window.localStorage.setItem(API_TOKEN_KEY, clean);
    }
  } catch {
    // Storage unavailable: nothing durable we can do.
  }
}

export function clearApiToken(): void {
  setApiToken('');
}

export function hasApiToken(): boolean {
  return getApiToken() !== '';
}

/** Stored tester token wins; otherwise whatever the session provides. */
export function resolveBearerToken(sessionToken: string | null | undefined): string {
  const stored = getApiToken();
  if (stored !== '') {
    return stored;
  }
  return sessionToken ?? '';
}

/** Pure header rule for the interceptor (unit-tested separately). */
export function withApiTokenHeader(
  existing: string | undefined,
  stored: string,
): string | undefined {
  if (stored !== '') {
    return `Bearer ${stored}`;
  }
  return existing;
}

/**
 * Build request headers attaching the Bearer token when one is present.
 * Suitable for native `fetch` requests across Knowledgeable API clients.
 */
export function getAuthHeaders(extra?: Record<string, string>): Record<string, string> {
  const token = getApiToken();
  const headers: Record<string, string> = { ...(extra ?? {}) };
  if (token !== '') {
    headers['Authorization'] = `Bearer ${token}`;
  }
  return headers;
}

let interceptorInstalled = false;

/**
 * Mirror the stored token into the global axios default. Upstream session
 * plumbing (`AuthContext` boot) writes its dummy token there, and the
 * fetch-based `authenticatedFetch` generation path reads it via
 * `getBearerToken()` — per-request axios interceptors never run for fetch,
 * so the default itself must carry the tester token.
 */
export function syncDefaultToken(): void {
  if (typeof window === 'undefined') {
    return;
  }
  const stored = getApiToken();
  try {
    if (stored === '') {
      delete axios.defaults.headers.common['Authorization'];
    } else {
      axios.defaults.headers.common['Authorization'] = `Bearer ${stored}`;
    }
  } catch {
    // Storage/shim edge: per-request headers still apply where interceptors run.
  }
}

/** Global axios interceptor: overwrite with the stored token, else untouched. */
export function installApiTokenInterceptor(): void {
  if (interceptorInstalled || typeof window === 'undefined') {
    return;
  }
  interceptorInstalled = true;
  axios.interceptors.request.use((config) => {
    const header = withApiTokenHeader(config.headers?.Authorization as string | undefined, getApiToken());
    if (header === undefined) {
      return config;
    }
    // Post-merge configs carry an AxiosHeaders instance; fall back to plain
    // assignment for the rare pre-normalization shape.
    const headers = config.headers as unknown as
      | { set(name: string, value: string): void }
      | Record<string, string>
      | undefined;
    if (headers && typeof (headers as { set?: unknown }).set === 'function') {
      (headers as { set(name: string, value: string): void }).set('Authorization', header);
    } else {
      config.headers = {
        ...((headers as Record<string, string> | undefined) ?? {}),
        Authorization: header,
      } as AxiosRequestHeaders;
    }
    syncDefaultToken();
    return config;
  });
}

// Module scope: capture `?ktoken=`, arm the interceptor, and seed the axios
// default (fetch-based paths read it) before the boot API calls. Imported
// from main.jsx (before App renders, hence before any request effect fires).
if (typeof window !== 'undefined') {
  getApiToken();
  installApiTokenInterceptor();
  syncDefaultToken();
}
