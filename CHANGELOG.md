# Changelog

## [Unreleased]
### Added (T17 — keyed-LLM validation with Gemini)
- Real-model turns verified: proactive `get_weak_dependencies`/`get_concept`/`log_observation` chains, Socratic explanations, and a clean repair pivot (weak Factor at 0.3 → tutor leads with factors before primes; healthy graph → direct explanation).
- Fixed tool-argument assembly: Gemini re-sends complete args per chunk, which concatenated into invalid JSON (broke execution AND caused follow-up 400s, since Gemini validates echoed args). Lenient first-value parsing + 3 unit tests.
- Removed hardcoded API keys from `crates/api/src/bin/` debug binaries (env-only now); `.env` is gitignored.
### Added (T15 — M5 proposals + atomic admission)
- Repaired `propose_concept`/`propose_relation` against the real candidate schema (prior code targeted nonexistent tables/columns, so every call failed) and added the missing tool definitions.
- Domain validation gates: 0.80 admission minimum, relation-type enum, no self-reference, resolvable endpoints (nodes or usable candidates); `ConceptRef` moved to domain.
- `admit_concept_candidate`: single-transaction node + accepted verdict + `graph_mutations` audit row, with gate re-check and persisted rejections.
- 9 candidate + 3 tool-loop tests green, including novel-material turns yielding pending-only candidates.
### Added (T11 — M7 chat highlighting, E2E-verified)
- Annotation store (`Map<messageId, ConceptAnnotation[]>`) + `concept_annotations` consume branches in both `useSSE` and `useResumableSSE` dispatchers.
- `remarkConceptHighlight` AST plugin (text-only, code/math/attribute-safe) + `<ConceptHighlight>` badges (known/weak/new + tooltip) wired through `markdownConfig`/`Markdown.tsx`.
- 8/8 live browser chat checks: send → stream → badge (`known`, `Prime Number · 98%`), no badge in KaTeX/code.

### Added (T12 — v2 generation-protocol adapter)
- `POST /api/agents/chat/:endpoint` negotiates v2 (header/body) → JSON start ticket; `GET /api/agents/chat/stream/:stream_id` serves snapshot+live SSE with resume convergence; `GET /api/agents/chat/status/:conversationId` authorizes terminal teardown.
- `GET /api/roles/:role_name` (USER fully granted — unblocks ChatRoute boot gate) and `endpointType: custom` fallback (unblocks `parseConvo`).
- Without these, browser chat boots to an empty main panel; API-level SSE alone is insufficient.
### Added (T9 — Graph Explorer)
- `GET /api/graph/neighborhood` Axum alias of canonical `GET /v1/graph/neighborhood` (Vite proxies `/api/*`; same controller/envelope; router-tested parity).
- Knowledge Graph side-panel in the vendored client (`graphTypes`/`graphUtils`/`api/graphClient`/`components/GraphExplorer` + self-sufficient `GraphPanel` container with root-UUID input, drill-down, review-only view); mounted via `useSideNavLinks` (`com_ui_knowledge_graph`); 28 Jest specs green; 10/10 live-browser CDP checks green against seeded backend.

### Fixed (vendoring repairs)
- Restored missing `apps/web/config/jest.workers.cjs` (verbatim upstream) so any Jest suite can run.
- Pinned `react-window@^1.8.10` via workspace `overrides` (fresh installs resolved v2.3.1, breaking `react-vtree`'s `FixedSizeList` import and the Vite dev server).
- Added missing `postcss-import` devDependency required by `client/postcss.config.cjs`.
- Built workspace packages (`data-schemas`, `data-provider`, `@librechat/client`) so `tsc`/Jest resolve; our files are type-clean (remaining 25 `tsc` errors are pre-existing upstream).

## [0.1.0] - 2026-09-10
### Added
- Reactive tutor tool-calling (weak dependency detection, observation logging, graph expansion).
- Robust SSE stream handling to fix OOM/CPU runaway issues.
- Comprehensive quality invariants (QUALITY.md) and test strategy (TEST_STRATEGY.md).
- Automated E2E test framework structure (Playwright/Chrome).
- Hardened CORS policy for local development.

### Fixed
- Fixed SSE stream reader recursive overflow.
- Resolved race conditions in frontend conversation state.
- Enabled tool-calling capabilities for Gemini integration.
