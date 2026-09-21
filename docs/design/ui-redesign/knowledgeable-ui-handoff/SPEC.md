# Knowledgeable UI redesign: implementation spec

Audience: the local coding agent. Read this whole file, then work phase by phase.
Owner's intent: the Map and Notebook panels are currently unstyled and unusable; the app also needs its own identity without diverging from upstream LibreChat.

## 0. Bundle contents and what is authoritative

| File | Role |
| :--- | :--- |
| `SPEC.md` (this file) | Behaviour, structure, file list, phases, acceptance checks. |
| `k.css` | Every colour, size and component style. **Copy it verbatim** to `apps/web/client/src/knowledgeable/styles/k.css`. Do not re-derive values. |
| `markup-reference.html` | The exact DOM and class names for the Map panel and the reading pane, rendered with only `k.css`. Open it in a browser. If your component looks different from this page in the same theme, the component is wrong. |
| `design-board.html` | Full visual direction (identity, palette, three screens). Open in a browser. Note: it draws a dim scrim behind the reading pane for illustration. **Ship without a scrim** (see 7.4). |
| `reference/*.png` | Target screenshots. Serif shown as Times because the fonts were not loadable when rendered; Literata replaces it. |
| `brand/mark.svg`, `brand/favicon.svg` | Logo mark and adaptive favicon. |

Order of authority: for how it looks, `markup-reference.html` + `k.css` win; for how it behaves, this file wins; the screenshots are for comparison.

Do not redesign anything. If a detail is missing, choose the most conservative option, record it in `docs/agent-context/roadmap_and_state.md`, and continue.

## 1. Ground rules

1. All new code lives in `apps/web/client/src/knowledgeable/`. Upstream files may only be touched at the seams named in each phase.
2. Style with the `k-` classes in `k.css`. Do not use Tailwind palette classes (`slate-*`, `emerald-*`, `amber-*`, `blue-*`) or raw hex in `knowledgeable/**`. Existing usages in `GraphCanvas.tsx` and `ConceptHighlight.tsx` are removed as part of this work.
3. Keep existing `data-testid` values wherever the behaviour still exists (section 10 lists what changes).
4. No new runtime dependencies except the font package in Phase 0.
5. Never call the backend differently. Data contracts (`graphClient`, `wikiClient`, types) are unchanged.
6. Learner-facing text never contains: UUID, Concept ID, Depth, Limit, "inspection", "neighborhood", "wiki" (use section 9's copy deck).
7. Each phase ends with a **gate**: run the tests, take the screenshots listed, and stop for review. Do not start the next phase before the gate passes.

## 2. Verify first (Phase 0 step 1)

Run these and write the answers to `docs/agent-context/ui-redesign-notes.md`. Later steps depend on them.

1. Colour variables: open `apps/web/client/src/style.css`. Record the exact variable names for surface/text/border roles and their **format** (hex, or RGB channel triplets such as `--surface-primary: 247 247 248`).
2. Runtime theme: search `packages/client/src/theme` (or wherever `ThemeProvider` / `applyTheme` lives). Record whether it writes CSS variables as **inline styles on `<html>`**. If yes, a stylesheet override of those variables will lose to it, and Phase 4 must go through the theme object instead (see 8.2).
3. Dark mode switch: confirm dark mode is the `dark` class on `<html>` (`tailwind.config.cjs` `darkMode: 'class'`). `k.css` assumes this.
4. Content globs: confirm `tailwind.config.cjs` scans `src/**/*`. (Not required for `k-` classes, but needed if any Tailwind utility is used for layout.)
5. Panel container: find how the side-panel content is wrapped (`Nav`/`SidePanel` components that render `NavLink.Component`). Record its width, padding, and whether it scrolls. The panels must not add their own outer padding twice; `k-panel` already has 16px.
6. Where the greeting, footer text "LibreChat 0.1.0 - Every AI for Everyone.", and composer sliders icon are produced (component names, i18n keys, or config flags).
7. Stacking: record upstream z-index for dialogs/popovers. `k-reader` uses `z-index: 60`; raise it only if it renders under something.
8. Fonts: confirm the package name `@fontsource-variable/literata` exists on npm and record the CSS `font-family` it registers (expected `Literata Variable`).

## 3. Phase 0: foundations (no visible change yet)

Files:
- Create `knowledgeable/styles/k.css` (copy from bundle).
- Create `knowledgeable/styles/index.ts`: imports `@fontsource-variable/literata` (weights 400 and 600 only if the package allows, otherwise the variable file) and `./k.css`.
- Upstream seam #1: import `~/knowledgeable/styles` **once**, after the upstream stylesheet import (`client/src/main.jsx` or `App.jsx`; record which). One line.
- Create `knowledgeable/components/ui/ConfidenceRing.tsx` (spec in section 5) and a Jest spec.

Gate: build passes, `ConfidenceRing` unit test passes, existing tests unchanged and green.

## 4. Shared primitives (Phase 1)

Create in `knowledgeable/components/ui/`. Keep each under about 60 lines. They are thin wrappers that apply `k-` classes; no logic.

| Component | Renders | Notes |
| :--- | :--- | :--- |
| `SearchField` | `<label class="k-field">` + icon + `<input class="k-input">` | props: `value,onChange,placeholder,ariaLabel,testId,onSubmit`. Enter submits. Include an icon button (`aria-label="Search"`, `data-testid` passed through) so existing tests that click a search button keep working. |
| `Button` | `<button class="k-btn [k-btn--primary|k-btn--ghost]">` | `type="button"` default. |
| `Segmented` | `<div class="k-seg" role="group">` + `<button aria-pressed>` | replaces the "Show needs-review only" checkbox. Options: `All N`, `Needs review N`. |
| `ConceptRow` | `<li><button class="k-row">` ring + `k-row__name` + `k-row__pct` | min height 46px; optional `sub` line; `aria-current` for the selected concept. |
| `EmptyState`, `ErrorState` | `k-empty`, `k-error` (`role="alert"`) | ErrorState takes `onRetry` and renders a `Button`. |

Gate: unit tests for each (renders, key handlers, aria attributes).

## 5. ConfidenceRing (Phase 0/1)

```tsx
// knowledgeable/components/ui/ConfidenceRing.tsx
import { confidenceStatus } from '../../graphUtils'; // 'healthy' | 'review' | 'unseen'

export interface ConfidenceRingProps { value: number | null | undefined; size?: number; label?: boolean }

const CLASS = { healthy: 'k-ring--solid', review: 'k-ring--building', unseen: 'k-ring--unseen' } as const;

export default function ConfidenceRing({ value, size = 22 }: ConfidenceRingProps) {
  const status = confidenceStatus(value ?? null);
  const v = status === 'unseen' ? 0 : Math.min(1, Math.max(0, value ?? 0));
  const sw = Math.max(2.2, size * 0.13);
  const r = (size - sw) / 2;
  const c = size / 2;
  const len = 2 * Math.PI * r;
  return (
    <svg className={`k-ring ${CLASS[status]}`} width={size} height={size} viewBox={`0 0 ${size} ${size}`} aria-hidden="true">
      <circle className="k-ring__track" cx={c} cy={c} r={r} strokeWidth={sw} />
      {v > 0 && (
        <circle className="k-ring__arc" cx={c} cy={c} r={r} strokeWidth={sw}
          strokeDasharray={`${(len * v).toFixed(2)} ${len.toFixed(2)}`} transform={`rotate(-90 ${c} ${c})`} />
      )}
    </svg>
  );
}
```

Rules:
- Arc length equals confidence. `>= 0.95` is Solid (already what `confidenceStatus` does; do not change the threshold). Below is Building. `null` is Not met yet (dashed track, no arc).
- The ring is decorative: the accessible name comes from the adjacent text ("Solid, 98%"). Add a helper `confidenceWords(value)` in `graphUtils.ts` returning `Solid | Building | Not met yet`, and a test.
- Sizes: 34 (focus summary), 22 (map focus pill and rows), 20 (map pills), 18 (chips).
- Confirm `confidenceStatus`'s real signature in `graphUtils.ts` before using it; adapt the call, not the threshold.

## 6. Map (Phase 2)

Replaces the Graph tab's presentation. Data logic in `GraphPanel.tsx` (abort controllers, load, search, drill-down) stays.

### 6.1 Panel structure (top to bottom)
Follow `markup-reference.html` exactly:
1. `k-panel__title` "Map".
2. `SearchField` "Find a concept". Results appear as a `k-rows` list directly below the field (max 8), replacing the focus area until one is picked or the field is cleared. Empty result: `EmptyState` "No concepts match that search."
3. `k-focus`: ring (34) + name + "Solid, 98%" (via `confidenceWords`) + `Button` "Open notes" (calls `openWiki(id)`).
4. `k-canvas` (see 6.2).
5. `k-legend`: "Arrows point to what a concept builds on."
6. `Segmented`: `All N` / `Needs review N`.
7. `ConceptRow` list sorted weakest first (`sortByConfidenceAscending`). Clicking a row selects it (same as clicking a node).
8. Developer controls: a `<details class="k-dev">` containing the existing UUID root input, Depth, Limit and Load button with their existing `data-testid`s. Render it **only** when `import.meta.env.DEV` or the URL has `?kdebug=1`. Keeps existing tests valid.
9. A `k-sr` element keeping `data-testid="graph-counts"` with the existing sentence ("N concepts · N links · N need review") for screen readers and tests.

Remove entirely: "or paste a Concept ID" as a visible control, the "Chat remains primary..." paragraph, the ASCII legend, the duplicated edge list, the "Graph Explorer" heading.

### 6.2 Default state
Currently the panel is empty until the user searches. New behaviour, in order:
1. If a concept was selected earlier this session (module-level variable in `store/mapSelection.ts`), load it.
2. Else `fetchMasteredConcepts()` and load the first item (server order is weakest first).
3. Else show `EmptyState`: "Nothing mapped yet. Ask the tutor about a topic and it will appear here."
Loading keeps the previous canvas visible and shows a `k-skeleton` only when there is no previous canvas.

### 6.3 Layout algorithm (new file `knowledgeable/mapLayout.ts`, pure, no React)
Signature: `layoutLayered(rootId, nodes, edges): { nodes: PlacedPill[]; edges: PlacedEdge[]; width: number; height: number }`.
Direction: **what a concept builds on sits below it.** The dependency edge `from depends_on to` means `to` is placed one rank lower than `from`.

Steps (deterministic, same input gives same output):
1. Consider only nodes in `nodes`. Dependency edges (`relation_type === 'Dependency'`) with both endpoints present define ranks; semantic edges never affect rank.
2. Rank 0 is the root. BFS from the root over dependency edges in both directions: going to a prerequisite (`to`) gives `rank + 1`, going to a dependent (`from`) gives `rank - 1`. First visit wins.
3. Repair: for at most `nodes.length` passes, for every dependency edge enforce `rank(to) >= rank(from) + 1` by raising `rank(to)`. This also breaks cycles safely (the pass cap stops it). Do not throw on cycles.
4. Nodes not reached (only semantic links, or filtered): give a semantic neighbour's rank if one is placed; otherwise put them in a final extra rank at the bottom.
5. Normalise so the smallest rank is 0.
6. Order within each rank: start alphabetical by name; then two barycentre sweeps (top-down using each node's neighbours in the rank above, then bottom-up); ties broken by name. This keeps order stable and edges short.
7. Pill width estimate: `clamp(96, 44 + name.length * 7.4, 220)` px for 13.5px Inter; focus pill `+ 20`. Names over 28 characters are truncated with an ellipsis (`title` keeps the full name).
8. Coordinates: rank gap 96px, horizontal gap 16px, rows centred on a common axis, padding 40px. `y = padding + rank * 96`. Width/height are the bounding box plus padding, never smaller than the container.
9. Edge routing (dependency): from the bottom-centre of the dependent pill to the top-centre of the prerequisite pill, cubic Bézier with control points at half the vertical distance (`M x1 y1 C x1 ym, x2 ym, x2 y2`), ending **3px above** the target so the arrowhead is visible. Marker id `k-arrowhead`, path class `k-arrow` (see `markup-reference.html`).
10. Semantic edges: `k-edge k-edge--related`, no arrowhead, drawn only for the hovered or selected node. Same-rank edges draw as a shallow arc between pill sides.
11. More than 30 nodes: switch to compact pills (ring plus truncated name, max width 140).

### 6.4 `MapCanvas.tsx` (replaces `GraphCanvas.tsx`)
- Scroll container `.k-canvas` (overflow auto) > `.k-canvas__stage` sized from the layout > `svg.k-edges` underneath + one `<button class="k-node">` per concept (HTML, not SVG text).
- Focus node gets `k-node--focus`. Each node shows `ConfidenceRing` (20; 22 for focus) and the label.
- DOM order is rank-major, left to right, so Tab order matches the picture. Enter and Space select; selection calls `onSelectConcept(id)` (drill-down as today).
- Initial scroll centres the focus node. `aria-label` on each node: "Name, solid" or "Name, building" or "Name, not met yet".
- Keep `GraphCanvas.tsx` until `MapCanvas` is wired; then delete it, `graphLayout.ts`, and their specs in the same commit.

Gate: layout unit tests (rank assignment, cycle safety, determinism, unreached nodes, edge endpoints outside pill boxes); component tests; screenshots at panel width 372px for the three-node case, in both themes, compared with `reference/map-*.png`.

## 7. Notebook and reading pane (Phase 3)

### 7.1 `WikiBrowser.tsx` (the Notebook tab)
- `k-panel__title` "Notebook"; `SearchField` "Search your notes" (local filter as today).
- `ConceptRow` list in server order (weakest first, never re-sorted). Row: ring + name + percent. If `wiki_status === 'stale'`, `sub` = "May be outdated".
- Empty: "No notes yet. Pages appear once you understand a concept well."
- No match: "No notes match that search."
- Truncated: "Showing the {n} weakest. Use the map to find others."
- Error: `ErrorState` with retry. Loading: three `k-skeleton` rows.

### 7.2 `WikiDrawer.tsx` (the reading pane)
- Outer element: `<aside class="k-reader" role="dialog" aria-label="Notebook page: {title}">` with `position: fixed` (from `k.css`). It must not depend on `ChatRoute`'s layout. This fixes the current "drawer pushes the chat" behaviour.
- Content order: close button; `h3.k-reader__title`; `k-reader__meta` (ring 22 + "Solid, 98%"); stale line if `is_stale` (`k-stale`: "May be outdated. It refreshes the next time you open it."); `p.k-lede` (the `summary`); `div.k-read` wrapping the existing `MarkdownBlocks`; "Builds on" chips; "Related" chips.
- If the generated markdown contains a line beginning "Check-for-understanding" (case-insensitive), render that paragraph in `div.k-try` with the label "Try this" instead of inside `k-read`. If not detected, render as normal text; do not change the backend prompt in this task.
- "Builds on" (`known_prerequisites`) and "Related" (`related_concepts`) become `k-chip` **buttons** that call `openWiki(concept_id)`. Prerequisites show a `ConfidenceRing`.
- Add a Back button (`Button` ghost, "Back") shown when the user navigated via a chip. Implement a small history stack inside `store/wikiDrawer.ts`; `closeWiki` clears it.
- Behaviour: Esc closes (already implemented, keep); on open, move focus to the close button; on close, restore focus to the element that opened it. Width `min(456px, 100vw)`.
- Loading: title skeleton + 4 body skeleton lines and the text "Writing your page. The first time can take a minute." Error and not-ready messages use `ErrorState`/`EmptyState` with the existing texts, softened per section 9.
- Wrap `MarkdownBlocks` in `.k-read` so it inherits the reading typography. Do **not** use Tailwind `prose` (the typography plugin is disabled upstream, so those classes do nothing).

### 7.3 Mounting
Replace `<WikiDrawer />` in `ChatRoute.tsx` with `<KnowledgeableHost />` (new file that renders `WikiDrawer` today and any future overlays). This is the only `ChatRoute` change.

### 7.4 No scrim
The board shows a dim overlay for illustration only. Ship without one so the sidebar stays clickable while a page is open. Esc and the close button dismiss.

Gate: component tests for stale, chips navigate, back, Esc, focus return; screenshot the open pane over an empty chat at 1280px and 390px, both themes, compared with `reference/notebook-*.png`.

## 8. Identity (Phases 4 and 5)

### 8.1 Concept badges in chat (small, part of Phase 4)
Replace the classes in `ConceptHighlight.tsx`:
- known: `k-concept k-concept--known`
- weak: `k-concept k-concept--weak`
- new: `k-concept k-concept--new`
- tooltip: `k-tip` (keep the existing structure, `data-testid`s and hover/focus behaviour)

### 8.2 App-wide colours
Goal: the whole app adopts the palette with near-zero upstream diff.
1. Create `knowledgeable/styles/brand.css`: for `:root` and `.dark`, override upstream's role variables (names and format found in section 2, items 1 and 2) with these values:

| Role | Light | Dark |
| :--- | :--- | :--- |
| surface-primary (canvas) | `#F4F6F5` | `#111D27` |
| surface-secondary (rail, sidebar) | `#EBEFED` | `#0D1720` |
| surface-tertiary (inputs, cards) | `#FFFFFF` | `#182733` |
| border-light | `#D5DDE0` | `#243746` |
| text-primary | `#14212B` | `#E6EDF2` |
| text-secondary | `#4A5B68` | `#A3B4C2` |
| text-tertiary | `#5A6B77` | `#7C8FA0` |

   Match upstream's value format (convert to triplets if that is what it uses). Map only roles that exist upstream; list any you could not find in the notes file.
2. If section 2 item 2 showed that `ThemeProvider` writes inline variables, a stylesheet override will not win. In that case do **not** fight it with `!important`. Instead pass a custom theme object through `ThemeProvider`'s supported prop in `App.jsx` (one seam) using the same values.
3. Compare screenshots of the chat screen before and after. Text and controls must keep at least 4.5:1 contrast (checked in section 11).

### 8.3 Chrome and branding (Phase 5, config first)
Prefer, in this order: (1) values returned by the Rust `/api/config` handler; (2) replacing static asset files; (3) an i18n override loaded from `knowledgeable/`; (4) a CSS rule keyed on a stable attribute; (5) a code edit. Record which one you used for each item.

| Item | Target | Suggested lever (verify) |
| :--- | :--- | :--- |
| Page title, app name | "Knowledgeable" | `appTitle` in `/api/config`, and `index.html` `<title>` |
| Footer "LibreChat 0.1.0 - Every AI for Everyone." | hidden | `customFooter` config or `interface` flag; else CSS |
| Favicon and logo | `brand/favicon.svg`, `brand/mark.svg` (add PNG exports of the mark at 16, 32, 180, 192, 512) | replace files in `client/public/assets/` and update `<link>`s in `index.html` |
| Landing greeting and robot icon | ring mark + "What do you want to understand?" | i18n string override, or the Landing component if no lever exists |
| Composer placeholder | "Ask about something you're learning" | i18n string override |
| Composer sliders icon | hidden | `interface` config; else CSS |
| Rail entries | Chats, Map, Notebook (keep bookmarks only if backed) | edit titles via i18n keys `com_ui_knowledge_graph` -> "Map", `com_ui_personal_wiki` -> "Notebook" |
| React Query devtools button | absent in production | confirm it is dev-only; if not, gate it |

Keep: model selector (restyled by tokens only), conversation list, message actions.

## 9. Copy deck

| Where | Old | New |
| :--- | :--- | :--- |
| Rail / panel title | Knowledge Graph, Graph Explorer | Map |
| Rail / panel title | Personal Wiki | Notebook |
| Search | Find a concept, or paste a Concept ID | Find a concept |
| Status words | healthy / review / unseen | Solid / Building / Not met yet |
| Filter | Show needs-review only | Needs review {n} (in the segmented control) |
| Legend | depends on (dependency), related (semantic) | Arrows point to what a concept builds on. |
| Panel hint | Search for a concept above to inspect its neighborhood... | Search for a concept to see what it builds on. |
| Panel footer | Chat remains primary; this panel is read-only inspection. | (removed) |
| Graph empty | No graph data yet. | Nothing mapped yet. Ask the tutor about a topic and it will appear here. |
| Graph load error | Could not load the graph neighborhood. | Couldn't load the map. Try again. |
| Graph service down | Graph service unavailable. Try again in a moment. | The map isn't available right now. Try again in a moment. |
| Concept not found | Concept not found. Check the ID and try again. | We couldn't find that concept. |
| Wiki service down | Wiki service unavailable. Try again in a moment. | Your notebook isn't available right now. Try again in a moment. |
| Wiki empty | No mastered concepts yet. Pages appear here once a concept reaches mastery. | No notes yet. Pages appear once you understand a concept well. |
| Wiki not ready | This concept isn't ready for a wiki page yet... | Notes appear once you have a good grip on this concept. Keep learning and check back. |
| Wiki loading | Loading wiki page... | Writing your page. The first time can take a minute. |
| Wiki stale | May be outdated, refreshes on next view. | May be outdated. It refreshes the next time you open it. |
| Wiki load error | Couldn't load the wiki page. | Couldn't load this page. Try again. |
| Truncated note | Showing the weakest N. Search the graph for more. | Showing the {n} weakest. Use the map to find others. |
| Node button | Wiki | Open notes |

Sentence case everywhere. No all-caps labels, no middle-dot strings in the UI, no arrows in link or button text.

## 10. Test impact

Existing specs encode the old presentation. Update them deliberately; do not delete assertions about behaviour.

| Spec | What breaks | Action |
| :--- | :--- | :--- |
| `GraphPanel.spec.tsx` | Uses `graph-root-input`, `graph-load`, `graph-counts` text, `graph-node-select-{id}`, `graph-search*`, `graph-retry`, alert texts | Keep testids. The root form lives in the `k-dev` details (rendered in tests via a prop or `?kdebug`). Keep `graph-counts` as `k-sr`. Update alert text expectations to the copy deck. Add tests for the default-state order (6.2). |
| `GraphCanvas.spec.tsx` | Expects SVG `<g>` nodes with `transform`, `marker-end` containing `graph-canvas-arrow`, `stroke-dasharray '5 4'`, `shortLabel` | Replace with `MapCanvas.spec.tsx`: node buttons, `data-status`, arrowhead present on dependency edges, related edges have class `k-edge--related` and no marker, click and keyboard select, renders nothing without nodes. Keep `shortLabel` only if still used. |
| `graphLayout.spec.ts` | Tests the radial BFS rings | Delete with `graphLayout.ts` when `MapCanvas` lands; new `mapLayout.spec.ts` covers section 6.3. |
| `GraphExplorer.spec.tsx` | Depends on the old explorer markup | Fold `GraphExplorer` into `GraphPanel` or reduce it to a presentational wrapper; port the assertions that still apply (filtering, sorting, wiki button). |
| `WikiBrowser.spec.tsx`, `WikiDrawer.spec.tsx` | Text and structure | Keep testids (`wiki-row`, `wiki-confidence`, `wiki-stale-mark`, `wiki-list`, `wiki-drawer`, `wiki-heading`, `wiki-title`, `wiki-summary`, `wiki-content`, `wiki-prereqs`, `wiki-related`, `wiki-close`, `wiki-retry`); update copy. Add: chips navigate, Back, focus return. |
| `ConceptHighlight.spec.tsx`, `highlightPipeline.spec.tsx` | Class names | Update to `k-concept--*`. Behaviour assertions stay. |

## 11. Definition of done (objective checks)

Add `e2e/knowledgeable-ui.spec.ts` (Playwright or the existing CDP harness) with these; failures block the phase.

1. **axe-core** (`@axe-core/playwright`, dev dependency) on the Map panel, Notebook panel and open reading pane, in light and dark: zero violations for `color-contrast` and `target-size`; zero serious or critical overall.
2. **No native-looking controls**: in dark theme every `input` inside `.k-panel` has a computed `background-color` other than `rgb(255, 255, 255)` and a text colour with at least 4.5:1 contrast against it.
3. **Minimum text size**: every visible text node in the panels and pane has computed `font-size >= 12px` (SVG text does not exist any more, so this is a plain DOM check).
4. **Arrowheads visible**: for each dependency edge in the map, the path's end point lies outside every `.k-node` bounding box (use `getBoundingClientRect`).
5. **Focus visible**: tabbing through the panel, each focused control has a non-`none` `outline-style`.
6. **Forbidden text**: none of `UUID`, `Concept ID`, `Depth`, `Limit`, `inspection`, `neighborhood` appears in visible text of the panels (outside `.k-dev`).
7. **No raw palette**: a Jest or shell check that `knowledgeable/**/*.{ts,tsx}` contains none of `/(slate|gray|emerald|amber|blue|red|green)-\d{2,3}/` and no `#[0-9a-fA-F]{3,8}` literals.
8. **Reading pane independence**: with the pane open, the chat area's bounding box is unchanged from when it is closed.
9. **Screenshots** saved to `docs/ui-proof/` at 1280x800 and 390x844, light and dark, for: Map with three concepts, Map empty, Map error, Notebook list, Notebook empty, reading pane open, chat landing. Attach them to the completion report. The owner approves them once; after that they are the regression baseline.
10. All Jest suites and `npm run build` pass, and `git diff --stat` for files outside `knowledgeable/` lists only the seams in this spec.

## 12. Phase plan and gates

| Phase | Deliverable | Upstream files touched | Gate |
| :--- | :--- | :--- | :--- |
| 0 | Verify notes, `k.css`, fonts, `ConfidenceRing` | 1 import line | build and tests green |
| 1 | Shared primitives | none | unit tests |
| 2 | Map: panel, layout, `MapCanvas`, default state | none | checks 1-6, 9 for Map; owner review of screenshots |
| 3 | Notebook and reading pane, `KnowledgeableHost` | `ChatRoute.tsx` (1 line) | checks 1-9 for Notebook and pane |
| 4 | Concept badges, app-wide tokens | 0-1 (theme seam) | contrast on chat screen, screenshots |
| 5 | Chrome and branding | config, assets, `index.html` | screenshot of landing, footer gone |

Report format after each phase: what changed (files), test counts, screenshots, and any decision you made where this spec was silent.

## 13. Out of scope

New backend routes or contract changes; changing the 0.95 threshold; new features (ask-tutor-about-this, sharing, editing notes); merging the Map and Notebook tabs; removing upstream files (hide, do not delete); changing tutor prompts.
