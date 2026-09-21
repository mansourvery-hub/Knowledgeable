/**
 * Branding copy contract (Phase 5, SPEC 8.3).
 *
 * The knowledgeable overrides must win over the bundled locale strings for
 * the rail titles and the composer placeholder.
 */
import { resources } from '~/locales/i18n';
import { knowledgeableOverrides } from '../i18n/overrides';

describe('knowledgeableOverrides', () => {
  it('carries the product voice', () => {
    expect(knowledgeableOverrides['com_ui_knowledge_graph']).toBe('Map');
    expect(knowledgeableOverrides['com_ui_personal_wiki']).toBe('Notebook');
    expect(knowledgeableOverrides['com_ui_composer_placeholder']).toBe(
      "Ask about something you're learning",
    );
  });

  it('wins over the bundled English strings', () => {
    const en = resources.en.translation as Record<string, string>;
    expect(en['com_ui_knowledge_graph']).toBe('Map');
    expect(en['com_ui_personal_wiki']).toBe('Notebook');
    expect(en['com_ui_composer_placeholder']).toBe("Ask about something you're learning");
  });
});
