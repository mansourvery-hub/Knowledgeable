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
  // F12: "Projects" is the wrong product word for a learning app — the
  // feature groups related conversations, so it reads "Folders".
  // Code identifiers, route paths, and query keys stay upstream-shaped.
  com_ui_add_first_project: 'Create a folder to keep related conversations together',
  com_ui_all_projects: 'All folders',
  com_ui_change_project: 'Change folder',
  com_ui_create_project: 'Create folder',
  com_ui_delete_project: 'Delete folder?',
  com_ui_delete_project_action: 'Delete folder',
  com_ui_delete_project_confirm: 'Delete "{{name}}"? The chats inside won\'t be deleted.',
  com_ui_edit_project: 'Edit folder',
  com_ui_new_project: 'New folder',
  com_ui_no_matching_projects: 'No folders match your search',
  com_ui_no_projects: 'No folders yet',
  com_ui_open_project: 'Open folder',
  com_ui_project: 'Folder',
  com_ui_project_count: '{{count}} folders',
  com_ui_project_count_partial: '{{count}}+ folders',
  com_ui_project_count_single: '1 folder',
  com_ui_project_create_error: 'Failed to create folder',
  com_ui_project_delete_error: 'Failed to delete folder',
  com_ui_project_name: 'Folder name',
  com_ui_project_name_placeholder: 'New folder',
  com_ui_project_not_found: 'Folder not found',
  com_ui_project_rename_error: 'Failed to rename folder',
  com_ui_project_update_error: 'Failed to update folder assignment',
  com_ui_project_updated: 'Folder assignment updated',
  com_ui_projects: 'Folders',
  com_ui_remove_from_project: 'Remove from folder',
  com_ui_search_projects: 'Search folders',
  com_ui_select_project: 'Select folder',
  com_ui_sort_projects_by: 'Sort folders by',
  com_ui_your_projects: 'Your folders',
};
