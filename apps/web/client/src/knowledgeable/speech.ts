/**
 * Knowledgeable product gate: speech (STT/TTS/voice mode) is FUTURE (policy
 * §9.4), not MVP. The Rust adapter serves no `/api/files/speech/config/*`
 * backend, and the init hook *enables* speech controls on exactly that
 * missing-configuration response — so every speech surface would be a dead
 * end: the composer mic button, per-message speak buttons, response
 * auto-play, and the SPEECH settings tab.
 *
 * Consumers (flip this to `false` for the Phase 5 speech project):
 * - `components/Chat/Input/ChatForm.tsx` does not mount `AudioRecorder`
 *   (mic) or `AutoPlayAudio`.
 * - `components/Chat/Messages/HoverButtons.tsx` does not render the speak
 *   button.
 * - `components/Nav/Settings/types.ts` hides the SPEECH tab, and every
 *   SPEECH entry in `components/Nav/Settings/registry.tsx` carries a `show`
 *   gate so settings search cannot surface them either.
 *
 * The components, hooks, and queries stay vendored in place behind these
 * gates so the future project reuses upstream code unchanged. Recoil
 * defaults and the init hook are deliberately untouched: render gates make
 * stored values unreachable without fighting user localStorage.
 */
export const speechDisabled = true;
