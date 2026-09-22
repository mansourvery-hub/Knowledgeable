/**
 * Knowledgeable product gate: the header model picker is a debug surface
 * (F16), not learner UI. Learners get one tutor; provider dispatch stays
 * server-side (M3) on the default model. The picker, its context, and the
 * selection state stay vendored in place behind this gate for debugging
 * and the future BYOK project.
 *
 * Consumers (flip `modelPickerDisabled` to `false` to restore the picker):
 * - `components/Chat/Header.tsx` does not mount `ModelSelector`.
 *
 * The `?kdebug` escape mirrors `showDevControls()` in `GraphPanel`: with it
 * present the picker mounts, so dispatch testing never needs a code change.
 */
export const modelPickerDisabled = true;

/** True when the header model picker should mount. */
export function showModelPicker(search?: string): boolean {
  if (!modelPickerDisabled) {
    return true;
  }
  const params =
    typeof search === 'string'
      ? new URLSearchParams(search)
      : typeof window !== 'undefined'
        ? new URLSearchParams(window.location.search)
        : new URLSearchParams();
  return params.has('kdebug');
}
