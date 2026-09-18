# Knowledgeable Frontend Extensions

All Knowledgeable-specific frontend code lives in this directory; upstream
LibreChat files stay untouched except at the narrow integration seams listed in
`docs/agent-context/integration/librechat.md`.

Planned contents (by milestone):

- `types.ts` — shared client-side Knowledgeable types (present).
- `graphTypes.ts` — neighborhood payload types mirroring `GraphService` (T9, present).
- `graphUtils.ts` — pure confidence/filter/sort helpers, no React deps (T9, present).
- `api/graphClient.ts` — typed `fetchNeighborhood` for `/api/graph/neighborhood` (T9, present).
- `components/GraphExplorer.tsx` — graph neighborhood panel (T9/M9, present).
- `components/GraphPanel.tsx` — self-sufficient side-panel container (T9, present).
- `store/annotations.ts` — per-message concept annotation store + SSE handler (T11/M7, present).
- `store/toolProgress.ts` — per-message tool activity store + SSE handler (T14/M4, present).
- `__tests__/` — Jest specs for the above (present).
- `plugins/remarkConceptHighlight.ts` — AST concept highlighting (M7, present).
- `components/ConceptHighlight.tsx` — rendered concept badges (M7, present).
- `components/ToolActivity.tsx` — inline tutor activity indicator (T14/M4, present).
- `components/WikiDrawer.tsx` — Personal Knowledge Wiki slide-over (M8).
- `api/` — typed clients for the Knowledgeable extension endpoints (M7+, present).
