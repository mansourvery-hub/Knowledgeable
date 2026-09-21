# UI Redesign Verification Notes (Phase 0 Step 1)

Date: 2026-09-21
Reference: `docs/design/ui-redesign/knowledgeable-ui-handoff/SPEC.md` Section 2

## 1. Colour Variables

- **File**: `apps/web/client/src/style.css`
- **Format**: Bare RGB channel triplets without commas or `rgb()` wrapper (e.g. `247 247 248`).
- **Tailwind integration**: As documented in `apps/web/packages/client/src/theme/utils/createTailwindColors.js`, tokens are wrapped by Tailwind as `rgb(var(--x) / <alpha-value>)`. Any plain CSS reference must wrap the channel triplet (e.g. `rgb(var(--text-primary))`).
- **Exact role variables in `html` (light mode)**:
  - `--surface-primary`: `var(--white)` (resolves to `255 255 255`)
  - `--surface-secondary`: `var(--gray-50)` (resolves to `247 247 248`)
  - `--surface-tertiary`: `var(--gray-100)` (resolves to `236 236 236`)
  - `--border-light`: `var(--gray-200)` (resolves to `227 227 227`)
  - `--text-primary`: `var(--gray-800)` (resolves to `33 33 33`)
  - `--text-secondary`: `var(--gray-600)` (resolves to `66 66 66`)
  - `--text-tertiary`: `var(--gray-500)` (resolves to `89 89 89`)
- **Exact role variables in `.dark` (dark mode)**:
  - `--surface-primary`: `var(--gray-900)` (resolves to `13 13 13`)
  - `--surface-secondary`: `var(--gray-800)` (resolves to `33 33 33`)
  - `--surface-tertiary`: `var(--gray-700)` (resolves to `47 47 47`)
  - `--border-light`: `var(--gray-700)` (resolves to `47 47 47`)
  - `--text-primary`: `var(--gray-100)` (resolves to `236 236 236`)
  - `--text-secondary`: `var(--gray-300)` (resolves to `205 205 205`)
  - `--text-tertiary`: `var(--gray-400)` (resolves to `153 150 150`)
- **Planned Phase 4 target mappings**:
  - `surface-primary`: Light `#F4F6F5` -> `244 246 245`, Dark `#111D27` -> `17 29 39`
  - `surface-secondary`: Light `#EBEFED` -> `235 239 237`, Dark `#0D1720` -> `13 23 32`
  - `surface-tertiary`: Light `#FFFFFF` -> `255 255 255`, Dark `#182733` -> `24 39 51`
  - `border-light`: Light `#D5DDE0` -> `213 221 224`, Dark `#243746` -> `36 55 70`
  - `text-primary`: Light `#14212B` -> `20 33 43`, Dark `#E6EDF2` -> `230 237 242`
  - `text-secondary`: Light `#4A5B68` -> `74 91 104`, Dark `#A3B4C2` -> `163 180 194`
  - `text-tertiary`: Light `#5A6B77` -> `90 107 119`, Dark `#7C8FA0` -> `124 143 160`

## 2. Runtime Theme System

- **Location**: `apps/web/packages/client/src/theme/context/ThemeProvider.tsx` and `apps/web/packages/client/src/theme/utils/applyTheme.ts`.
- **Inline styles on `<html>`**: **YES**.
  - `ThemeProvider.tsx` calls `applyResolvedTheme(resolveTheme(definition, mode), root)` (or `applyTheme(legacyThemeRGB, root, ...)`), which iterates through all theme variables and executes:
    `root.style.setProperty(property, value)` where `root = document.documentElement` (`<html>`).
- **Consequence for Phase 4**: Because inline styles on `<html>` have higher cascade specificity than stylesheet rules (`:root` / `html`), stylesheet overrides in CSS will lose to the runtime theme engine. Phase 4 must provide the theme variables through the theme object/definition in `ThemeProvider` (`themeDefinition` prop or custom theme provider seam), exactly as specified in SPEC.md section 8.2 item 2.

## 3. Dark Mode Switch

- **Tailwind configuration**: `apps/web/client/tailwind.config.cjs` line 15: `darkMode: ['class']`.
- **DOM toggling**: `ThemeProvider.tsx` line 612: `root.classList.toggle('dark', mode === 'dark')`.
- **Confirmation**: Dark mode is strictly controlled via the `dark` class on `<html>` (`document.documentElement`). `k.css` selector `.dark` directly aligns with this mechanism.

## 4. Content Globs

- **Tailwind configuration**: `apps/web/client/tailwind.config.cjs` lines 9-13:
  ```js
  content: [
    './src/**/*.{js,jsx,ts,tsx}',
    '../packages/client/src/**/*.{js,jsx,ts,tsx}',
  ]
  ```
- **Confirmation**: Confirmed that `tailwind.config.cjs` scans `./src/**/*.{js,jsx,ts,tsx}` recursively, covering `src/knowledgeable/**`.

## 5. Panel Container

- **Files**:
  - `apps/web/client/src/components/UnifiedSidebar/Sidebar.tsx`
  - `apps/web/client/src/components/SidePanel/Nav.tsx`
  - `apps/web/client/src/components/UnifiedSidebar/constants.ts`
- **Wrapping structure**:
  - Desktop sidebar renders `<ExpandedPanel links={links} ... />` (rail, width 52px via `COLLAPSED_WIDTH`) alongside:
    `<nav className="min-h-0 flex-1 overflow-hidden bg-surface-primary-alt ..."><SidePanelNav links={links} /></nav>`.
  - `SidePanelNav` (`Nav.tsx`) renders:
    `<div className="flex h-full min-h-0 flex-col overflow-y-auto overflow-x-hidden text-text-primary">{...}</div>`.
- **Dimensions & Scrolling**:
  - **Width**: `flex-1` within `aside` (sidebar total width: `EXPANDED_MIN` = 360px up to 40% viewport, minus 52px rail = ~308px minimum width, or 372px in the design board).
  - **Padding**: **0px** (neither `nav` in `Sidebar.tsx` nor the `div` in `Nav.tsx` adds padding).
  - **Scrolling**: `Nav.tsx` has `overflow-y-auto overflow-x-hidden`, so the container handles vertical scroll.
  - **Double-padding verification**: The container adds 0 outer padding. `k-panel` having `padding: 16px;` is correct and does not duplicate any container padding.

## 6. Greeting, Footer, and Composer Sliders Icon

- **Greeting**:
  - Component: `apps/web/client/src/components/Chat/Landing.tsx` (lines 133-136).
  - Logic: Evaluates `greetingText = isTemporary ? localize('com_ui_temporary') : (resolvedWelcome ?? scheduledGreeting)`.
  - Scheduled greeting: `useGreeting(user?.name)` via `apps/web/client/src/utils/greeting.ts` using i18n keys `com_ui_greeting_*`.
  - Config flag lever: `startupConfig.interface.customWelcome` (string with optional `{{user.name}}`).
  - Icon: Rendered as `ConvoIcon` (size 41) or `HatGlasses` (temporary) at lines 145-161 of `Landing.tsx`.
- **Footer text ("LibreChat 0.1.0 - Every AI for Everyone.")**:
  - Component: `apps/web/client/src/components/Chat/Footer.tsx` (lines 79-85).
  - Logic: `genericFooter = configuredOnly ? '' : '[LibreChat ' + Constants.VERSION + '](https://librechat.ai) - ' + localize('com_ui_latest_footer')`.
  - i18n key: `com_ui_latest_footer`.
  - Config flag lever: `startupConfig.customFooter`. If configured as `""` (empty string) in `/api/config`, `Footer.tsx` suppresses the footer line completely.
- **Composer Sliders Icon**:
  - Component 1: `apps/web/client/src/components/Chat/Input/ToolsDropdown.tsx` (lines 381-394, `id="tools-dropdown-button"`, rendering Lucide `<Settings2 className="size-5" />`) mounted in `BadgeRow.tsx`.
  - Component 2: `apps/web/client/src/components/Chat/Input/HeaderOptions.tsx` (lines 47-60, `id="parameters-button"`, rendering Lucide `<Settings2 size={16} />`).
  - Config flag lever: `HeaderOptions` is gated by `interfaceConfig?.parameters === true`. `ToolsDropdown` can be hidden via tools configuration or CSS.

## 7. Stacking Order (Z-Index)

- **Existing layer values**:
  - Base document / chat area: `Root.tsx` is `relative z-0`.
  - Headers / bars: `Header.tsx` is `z-10`.
  - Banners: `z-20`.
  - Submenus / Popovers: `z-40`, `z-50`, `z-[61]`.
  - Options popover: `OptionsPopover.tsx` is `z-[70]`.
  - Mobile drawer: `DRAWER_Z_INDEX = 110`.
  - Floating menus / tokens: `z-[125]`, `z-[200]`.
  - Modals / Radix Dialogs: `Dialog.tsx` is `z-[999]`.
  - Toast Viewport: `RadixToast.Viewport` is `z-[1000]`.
  - Drag-drop overlays: `z-[9998]`, `z-[9999]`.
- **Evaluation for `k-reader` (`z-index: 60`)**:
  - At `z-index: 60`, `k-reader` renders above normal chat content (`z-0`), headers (`z-10`), and toolbars.
  - It stays underneath modal dialogs (`z-[999]`), alerts, and toasts (`z-[1000]`), which is the desired behaviour for a slide-over panel. No z-index adjustment is needed.

## 8. Fonts

- **NPM Package**: `@fontsource-variable/literata` confirmed exists on npm (latest version 5.3.0).
- **CSS Font Family Registered**: `@font-face { font-family: 'Literata Variable'; ... }` with weights `200 900`.
- **Alignment**: Exactly matches `--k-font-read: 'Literata Variable', 'Literata', Georgia, 'Times New Roman', serif;` in `k.css`.

## 9. Phase 1 decisions (SPEC silent details, conservative options)

Recorded here rather than `roadmap_and_state.md` to avoid colliding with
that file's unrelated T24 phase numbering.

- **SearchField**: icon button placed before the input inside `label.k-field`
  (single icon; the button IS the icon). Button `data-testid` derived from
  the input prop by stripping a trailing `-input`
  (`graph-search-input` -> `graph-search`) so Phase 2 wiring keeps existing
  suites green with one prop. Renders the label only, no `<form>`; the
  parent keeps the form wrapper and `graph-search-form` testid in Phase 2.
  Enter (input `onKeyDown`) and icon click both call `onSubmit`.
- **Button**: `variant` defaults to plain `k-btn`; `primary`/`ghost` add the
  modifier class. `type` defaults to `button`.
- **Segmented**: generic `allCount`/`reviewCount`/`selected`/`onSelect` props;
  default group `aria-label` is `Filter` per `markup-reference.html`.
- **ConceptRow**: no `k-row__body` wrapper when `sub` is absent (exact markup
  contract); wrapper present only with `sub`. Percent always rendered via
  `formatConfidence` (unseen renders as `unseen`). `title` carries the full
  name for truncated labels. No `aria-label` on the row button; the
  accessible name comes from visible name + percent text.
- **ErrorState**: retry button `data-testid` via explicit `retryTestId` prop
  (Phase 2/3 pass `graph-retry`/`wiki-retry` through); default retry label
  `Try again`. `role="alert"` on the `k-error` container.
- **Forbidden-pattern check (§11.7)**: new `ui/` files are clean (no palette
  classes, no hex). Remaining hits are pre-existing `GraphCanvas.tsx` /
  `ConceptHighlight.tsx` usages, removed as part of Phases 2/4 per SPEC 1.2.

## 10. Phase 2 decisions (SPEC silent details, conservative options)

- **GraphExplorer folded into GraphPanel** (SPEC 10 allows fold or wrapper):
  `GraphExplorer.tsx` deleted; filtering/sorting/wiki-button assertions
  ported into `GraphPanel.spec.tsx` (weakest-first order, Segmented filter,
  `Open notes` via `openWiki`). `GraphCanvas.tsx`, `graphLayout.ts` and all
  three old specs deleted in the same wiring commit per SPEC 6.4.
- **mapSelection**: plain module variable, no subscription; the panel reads
  it once on mount for the default state.
- **Mastered-list failure degrades to the empty state** (search stays
  available); no error is shown for the background boot fetch.
- **Search results use plain `k-row` name buttons**, not `ConceptRow`:
  search hits carry no confidence, so a ring/percent would be invented data.
- **Malformed-UUID client message reuses the not-found copy**
  ("We couldn't find that concept.") to keep `UUID` out of visible panel
  text (§11.6; the dev form itself stays inside exempt `.k-dev`).
- **Layout extras**: focus `+20` applied before the 220 clamp; compact lower
  bound 64 (spec pins only the 140 max); truncation `slice(0, 27) + '…'`
  mirroring the old `shortLabel` pattern (`shortLabel` not kept); different-
  rank semantic edges draw as straight centre-clipped lines (spec pins only
  same-rank arcs); `minSize` 4th param floors the stage on the measured
  container ("never smaller than the container"), defaulting to zero.
- **MapCanvas**: measures the scroll container with `ResizeObserver`,
  guarded on the `observe` method (jsdom ships a stub without it); initial
  scroll centres focus via `scrollLeft`/`scrollTop` assignment (jsdom-safe).
  Enter activates natively; Space is manual with `preventDefault` (no
  double-fire). Node `aria-label`s use lowercase words ("Name, solid");
  visible copy keeps `confidenceWords`.
- **Screenshots**: desktop browser unavailable, so proof shots were taken
  with headless Chrome against a temporary fetch-stubbed harness (deleted
  after; three-node case at 372px, both themes, stored under
  `docs/ui-proof/map-panel-372-*.png`). The collapsed `Developer controls`
  row in the shots is dev-only (`import.meta.env.DEV`, hidden in prod).

## 11. Phase 3 decisions (SPEC silent details, conservative options)

- **Percentages always visible in Notebook/reader** (SPEC 7.1/7.2, markup,
  board): this overrides the F7/F8 debug-only gating for these two
  surfaces — the gated Phase 1 `ConceptRow` renders pct unconditionally by
  design. The debug toggle still governs chat badges/tooltips. The two
  WikiBrowser debug tests were replaced with an always-visible assertion.
- **`wiki-heading` dropped**: the markup contract has no `h2`; looks defer
  to `markup-reference.html` over the old testid table.
- **`k-read` added alongside `markdown message-content`** (kept so chat
  component styling still applies); only the no-op `prose` classes were
  removed per SPEC 7.2.
- **Try-this section renders through the same `MarkdownBlocks` pipeline**
  so concept badges still highlight inside it.
- **History records every `openWiki` transition**, not only chip clicks
  (single entry point; "Open notes" navigation behaves the same).
- **Focus moves to close only on initial open**, not on every chip
  navigation; restore-on-close targets the recorded opener.
- **Not-ready stays `ErrorState` with retry** (keeps the existing retry
  flow), copy softened per the deck.
- **Screenshots**: headless-Chrome harness as in Phase 2 (deleted after).
  The badge tooltips needed the upstream stylesheets in the harness to
  hide — production-real, since `main.jsx` loads them before `k.css`.

## 12. Phase 4 decisions (SPEC silent details, conservative options)

- **No `brand.css`**: the verify notes proved inline-variable theming, so
  SPEC 8.2 option 2 applies directly — palette travels as `themeDefinition`.
- **`themeDefinition`, not `themeRGB`**: `fromLegacyTheme` pins one palette
  to both modes; only a per-mode definition carries light + dark values.
  `resolveTheme` merges the seven roles over the upstream bases, so all
  other tokens (submit greens, destructive reds, syntax, series) are
  untouched. Appearance-mode switching (incl. the mode-only selector) keeps
  working; no custom-color UI exists to conflict with.
- **Env-theme spread kept as-is** in `App.jsx`: the provider prefers a valid
  definition, so the seam is one import + one prop with zero re-plumbing.
- **Tooltip keeps positioning/visibility utilities** (`invisible absolute …
  group-hover:visible …`); only the cosmetic ones `k-tip` owns were
  dropped. Structure, testids, and hover/focus behaviour unchanged.
- **Contrast locked in CI** (`brandTheme.spec.ts` computes all 18
  text/surface pairs): light min 4.76, dark min 4.57.
- **Screenshots**: swatch before/after per mode proves the seam end to end
  (`brand-swatch-*-{before,after}.png`); `reader-badges-light.png` proves
  the `k-concept` badge identity in real markdown. Harness deleted after.
