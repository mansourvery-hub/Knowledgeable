/**
 * Shared debug escape (F18): debug-only surfaces stay fully working for
 * development but never render in the shipped product unless explicitly
 * invoked. No code is deleted — gates only control mounting/listing.
 *
 * Mirrors the pre-existing `?kdebug` convention (`showDevControls` in
 * `GraphPanel`, `showModelPicker`); centralized here so new gates share
 * one tested helper instead of re-reading `window.location`.
 */
export function hasDebugParam(search?: string): boolean {
  const params =
    typeof search === 'string'
      ? new URLSearchParams(search)
      : typeof window !== 'undefined'
        ? new URLSearchParams(window.location.search)
        : new URLSearchParams();
  return params.has('kdebug');
}
