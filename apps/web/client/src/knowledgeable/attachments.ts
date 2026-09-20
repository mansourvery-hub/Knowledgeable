/**
 * Knowledgeable product gate: file attachments/uploads are FUTURE (policy
 * §9.4), not MVP. The Rust adapter serves no `/api/files/*` backend, so every
 * upload surface would be a dead end: the composer attach button, drag-drop,
 * paste-as-file, the Files side-panel entry, and the Manage Files settings
 * entry.
 *
 * Consumers (flip this to `false` for the Phase 5 attachments project):
 * - `hooks/Files/useUploadOptions.ts` forces `uploadsDisabled`, which the
 *   paste, drag, and modal flows already honor with the upstream disabled
 *   toast — one line covers all three consistently.
 * - `components/Chat/Input/ChatForm.tsx` does not mount `AttachFileChat`.
 * - `hooks/Nav/useSideNavLinks.ts` skips the Files panel entry.
 * - `components/Nav/Settings/registry.tsx` hides the Manage Files entry.
 *
 * The panel, dialogs, and data-provider queries stay vendored in place behind
 * these gates so the future project reuses upstream code unchanged.
 */
export const attachmentsDisabled = true;
