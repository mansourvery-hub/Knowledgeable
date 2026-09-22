/**
 * Knowledgeable product copy overrides (Phase 5, SPEC 8.3, lever 3).
 *
 * Merged over the upstream locale bundles in `locales/i18n.ts` so rail
 * titles and the composer placeholder carry the product voice without
 * forking the locale files. Keys are upstream keys with new values, except
 * `com_ui_composer_placeholder`, which is ours (the generic
 * `com_endpoint_message_new` also feeds screen-reader message labels, so it
 * must keep its "Message …" shape).
 */
export const knowledgeableOverrides: Record<string, string> = {
  com_ui_knowledge_graph: 'Concept Map',
  com_ui_personal_wiki: 'Wiki',
  com_ui_composer_placeholder: "Ask about something you're learning",
};
