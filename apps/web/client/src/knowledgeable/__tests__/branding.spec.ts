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
    expect(knowledgeableOverrides['com_ui_knowledge_graph']).toBe('Concept Map');
    expect(knowledgeableOverrides['com_ui_personal_wiki']).toBe('Wiki');
    expect(knowledgeableOverrides['com_ui_composer_placeholder']).toBe(
      "Ask about something you're learning",
    );
  });

  it('wins over the bundled English strings', () => {
    const en = resources.en.translation as Record<string, string>;
    expect(en['com_ui_knowledge_graph']).toBe('Concept Map');
    expect(en['com_ui_personal_wiki']).toBe('Wiki');
    expect(en['com_ui_composer_placeholder']).toBe("Ask about something you're learning");
  });

  it('calls conversation groups Folders, never Projects', () => {
    expect(knowledgeableOverrides['com_ui_projects']).toBe('Folders');
    expect(knowledgeableOverrides['com_ui_all_projects']).toBe('All folders');
    expect(knowledgeableOverrides['com_ui_your_projects']).toBe('Your folders');
    expect(knowledgeableOverrides['com_ui_new_project']).toBe('New folder');
    expect(knowledgeableOverrides['com_ui_no_projects']).toBe('No folders yet');
    const en = resources.en.translation as Record<string, string>;
    expect(en['com_ui_projects']).toBe('Folders');
    expect(en['com_ui_all_projects']).toBe('All folders');
    const projectKeys = Object.keys(en).filter(
      (k) => /project/i.test(k) && !/langfuse|schedule/i.test(k),
    );
    for (const key of projectKeys) {
      expect(en[key]).not.toMatch(/[Pp]roject/);
    }
  });
});
