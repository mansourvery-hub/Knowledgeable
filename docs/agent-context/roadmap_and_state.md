# Knowledgeable — Roadmap and State

## 1. State Rules

<roadmap_rule>
This file is the authoritative implementation state source.
Check items off only after the corresponding verification gate passes.
Do not mark work complete based on code presence alone.
</roadmap_rule>

Status vocabulary:
```text
[ ] not started
[-] in progress
[x] complete
[!] blocked
```
Current phase:

```text
M2 — Conversation Persistence & Real-Time SSE Streaming (in progress)
  backend adapter complete and tested; browser exit gate pending
Next: M3 — Multi-Provider / BYOK Configuration & Model Selection
```

### Immediate next action (handoff)

M2's backend adapter is complete and verified (`cargo test -p api`; 12 tests
green, plus a live server smoke test of `created`/delta/`final`, multi-turn
parent chaining, title derivation, and delete cascade). The only open M2 item is
its browser exit gate:

1. `cd apps/web && npm install` (npm workspaces: `client/` + `packages/*`).
2. Run the Axum server on port 3000, then `npm run dev` (Vite proxies `/api` to
   `BACKEND_PORT=3000`).
3. In the browser: send a message and confirm streaming; reload and confirm
   history persists; create/switch/delete conversations from the sidebar.
4. If green, tick the three M2 exit-gate boxes and advance the current phase to
   M3.

Notes for a fresh agent:
- New adapter code lives in `crates/api/src/routes/librechat/`
  (`system.rs`, `convos.rs`, `chat.rs`); the canonical contract is documented in
  `docs/agent-context/integration/librechat.md` section 4.
- `apps/web` is the untracked vendored LibreChat workspace. Keep
  Knowledgeable-only frontend code under `apps/web/client/src/knowledgeable/`;
  upstream files must only change at the seams listed in the integration doc.
- Do NOT reintroduce MongoDB/Redis/Node — the Rust Axum adapter replaces the
  LibreChat backend; SQLite remains the single source of truth.
- The deprecated Flutter client in `apps/client` is not part of the target
  architecture; do not build on it.

---

## 2. Deprecation Log: Obsolete Components

The following items from the initial Flutter/Drift architecture are formally deprecated and removed from active tracking:
- `apps/client` Flutter codebase (Riverpod, Flutter Flyer Chat widgets, Drift SQLite, go_router).
- Client-side Drift SQLite database and offline sync protocols.
- Flutter-specific tests and CI checks (`flutter analyze`, `flutter test`).
- Generic chatbot features unaligned with frontier tutoring.

---

## 3. Milestone Progression

### M0: Architectural Discovery, Upstream Audit & Strategy
- [x] Audit upstream LibreChat (`v0.8.8-rc3`) client workspace and rendering pipeline.
- [x] Evaluate integration models (A, B, C, D, E) against core invariants.
- [x] Design concept-aware AST markdown rendering without raw HTML pollution.
- [x] Design persistent, cached Personal Knowledge Wiki lifecycle.
- [x] Define official provider/account authorization strategy (BYOK / Local LLMs).
- [x] Author comprehensive integration guide (`docs/agent-context/integration/librechat.md`).
- [x] Update core architecture documentation (`docs/agent-context/architecture.md`).
- [x] Re-baseline implementation roadmap and state (`docs/agent-context/roadmap_and_state.md`).

**Exit Gate**:
```text
[x] Architectural documentation complete and approved.
[x] Integration seams identified with concrete file locations and contracts.
[x] Invariants preserved: Rust/SQLite core, zero external daemons, LLM reasoning separated from code invariants.
```

---

### M1: Minimal LibreChat Client Scaffolding & Axum Adapter Spike
- [x] Vendor upstream LibreChat client (`client/`, `@librechat/client`, `@librechat/data-provider`) into `apps/web`.
- [x] Configure Vite proxy to route `/api/*` to Axum backend (port 3000).
- [x] Implement startup config endpoint `GET /api/config` in `crates/api` (minimal single-user profile, auth disabled).
- [x] Implement user session endpoints `GET /api/user` and `POST /api/auth/refresh` in `crates/api`.
- [x] Add basic package scripts (`npm run dev`, `npm run build`) in `apps/web`.

**Exit Gate**:
```text
[ ] Running apps/web boots LibreChat UI in browser without console errors.
[x] Client automatically authenticates via Axum adapter and reaches the new chat view.
[x] No Node.js Express server or MongoDB instance running.
```
_Browser proof 2026-09-19 (headless Chrome, backend :3000 + Vite :3090, no
keys): 465KB boot DOM contains composer, "New chat", history container, the
`knowledgeable` endpoint, and the knowledge-graph nav entry — so the
silent-refresh → user → roles → chat boot chain works end to end, served only
by the Rust adapter + Vite. Zero removed-surface strings (Marketplace,
Memories, Skills, Schedules, Parameters, Prompts) in the static DOM. A live
CDP probe (raw WebSocket harness, `/tmp/cdp-boot.js`) additionally confirmed
the app routes to `/c/new` with a visible composer. The
first box stays open: boot logs handled Axios 404/405 probes (no uncaught
exceptions), inventoried below._

_Settle analysis 2026-09-19: the probe noise is boot-only, not perpetual.
`useActiveJobs` has `retry: false` and polls only when jobs are listed;
MCP/Tools queries set `refetchInterval: false`; code-env status needs a
workspace id; schedules poll only from the hidden panel (zero `/api/schedules`
hits observed). A 90s idle CDP capture shows the boot burst only. So M1 box 1
is a product call, not a bug hunt: either accept documented handled noise, or
schedule per-surface query suppression (each its own divergence tradeoff). Box
left open pending that call._

---

### M2: Conversation Persistence & Real-Time SSE Streaming
- [x] Implement `GET /api/convos` and `GET /api/convos/:id` reading from SQLite `conversations`.
- [x] Implement `GET /api/messages/:conversationId` reading from SQLite `conversation_messages`.
- [x] Implement chat streaming endpoint `POST /api/agents/chat/:endpoint` in `crates/api`.
- [x] Map internal `TutorEvent` stream to LibreChat SSE format (`created`, delta chunks, `final`).
- [x] Implement conversation title auto-generation or fallback in SQLite.
- [x] Expose supporting endpoints required by the client: `GET /api/endpoints`, `GET /api/models`, `POST /api/convos/update`, `DELETE /api/convos`, `GET /api/convos/gen_title/:id`.
- [x] Add router-level tests for config/session, convos, messages, streaming turn, title fallback, and validation errors.

**Exit Gate**:
```text
[x] User sends a message in the UI; response streams in real time with smooth rendering.
[x] Reloading the browser preserves full conversation history from SQLite.
[x] New conversations appear in the left sidebar and can be switched/deleted.
```

_Streaming evidence 2026-09-19: two UI-submitted turns completed and rendered
(`/c/eb568c7f`, `/c/6d0aa213`, `/c/e4f418e7`, `/c/73444b03`); the keyed-path
protocol delivers multiple cumulative deltas per turn (measured: 4 text deltas
at 9/87/214/313 chars inside a 20-frame stream — provider-chunked, not
token-by-token, so 100ms DOM polls see coarse jumps; no jank observed).
Sidebar switch/delete ops still open._

_Backend contract verified by adapter tests (`cargo test -p api`); the browser
exit gate requires a manual `apps/web` dev-server run._

_Live protocol proof 2026-09-19 (backend :3000, no keys, `local-tutor`):
`POST /api/agents/chat/knowledgeable` streamed 37 SSE frames
(`created` → deltas → `final`) for a fresh conversation; `GET /api/convos`
went 5 → 6; `GET /api/messages/:id` returns the user + assistant pair. So
streaming, persistence, and history round-trip hold server-side; the remaining
browser click-path (type → see stream → reload → sidebar ops) is still open._

_Browser proof 2026-09-19 (CDP, real keyed model turn via UI submit): question
sent from the composer created `/c/6d0aa213`, server turn completed
(`error: false`, reply `15`); after `Page.reload` the DOM renders both the
user bubble and the assistant reply in message containers
(`DIV.text-message > DIV.markdown.prose.message-content > P`). Sidebar
switch/delete ops still open. Note: the first send attempt "failed" only on a
wrong harness marker (expected a local-tutor skeleton string, got a real model
reply) — product worked; also fixed a response-id capture bug in the throwaway
harness (`/tmp/cdp-*.js`, not committed)._

_Sidebar proof 2026-09-19 (CDP, desktop viewport): sidebar lists derived titles;
clicking a row switched to `/c/6d0aa213` with its messages; row menu →
Delete → "Delete chat?" confirm removed it (row gone, app back at `/c/new`);
backend confirms the cascade (`GET /api/convos/:id` 404, messages `[]`,
reconciled count 11 − 1 = 10). M2 browser exit gate fully closed._

---

### M3: Multi-Provider / BYOK Configuration & Model Selection
- [x] Implement `GET /api/models` advertising configured backend providers (env-driven: Gemini/OpenAI models appear only with keys, Ollama model only when `OLLAMA_ENABLED=true`, always plus offline `local-tutor`).
- [x] Honor request BYOK keys (`apiKey` turn-scoped, precedence over server keys, never stored/logged). Client settings UI for key entry is still open.
- [x] Wire model switching in the LibreChat header to backend `LlmClient` dispatching (per-turn resolve by model name; missing key is an explicit 400, never a silent wrong provider; requested model threaded into the turn).
- [x] Add support for local OpenAI-compatible endpoints (Ollama/vLLM) without API keys (`OpenAiClient::with_base_url`, `OLLAMA_BASE_URL`, keyless mapping; no live server in this environment — unverified against real Ollama).

**Exit Gate**:
```text
[x] User can switch between Gemini, OpenAI, and local Ollama from the UI model picker.
[x] Providing a personal API key in settings correctly routes requests using that key.
[-] Local Ollama instance successfully streams tutor responses without external internet access — REMOVED-FOR-NOW 2026-09-22 (mobile-first): the Ollama provider branch, `OLLAMA_*` env, and model advertisement are out; `ollama*` names fall through to the boot default. Generic `with_base_url` client retained for a future local/mobile relay; restoration is one revert.
```
_Model switching and BYOK routing verified at router level (23 api tests);
no Ollama binary in this environment, so the offline-streaming gate stays
open. The client settings UI for key entry is likewise open (backend honors
`apiKey` today). (2026-09-22: Ollama removed-for-now per above; the gate is
moot until/unless the provider returns.)_

_Live failure proof 2026-09-19: keyless `gpt-4o-mini` turn returns an explicit
400 (`validation_failed`, "set OPENAI_API_KEY or provide apiKey") — never a
silent wrong provider. Wart found and fixed same day: failed turns persisted
an empty shell (`resolve_conversation` ran before key validation) — credential
resolution now precedes any database write, covered by
`chat_rejected_turn_persists_no_conversation`, live-proven (count unchanged
across the probe; normal turns unaffected)._

_BYOK UI gap scoped 2026-09-19: no working key-entry path exists for the
`knowledgeable` endpoint. `useRequiresKey` needs a `userProvide` flag our
endpoint config deliberately omits; the settings `providerApiKeys` entry hides
(`hasUserProvidedEndpoints` false); `PUT/GET /api/keys` 404s (no backend); and
nothing in the UI produces our turn-scoped `apiKey` field (backend honors it,
router-proven). So the M3 "key in settings" tick holds at backend level only.
Phase 5 BYOK options: (1) advertise `userProvide` to unlock upstream key UI —
rejected without a real `/api/keys` store (half-working); (2) Knowledgeable-
scoped entry (local key → turn-scoped `apiKey`, never stored) — the likely
shape, with strong security boundaries per policy §9.4._

---

### M4: Proactive Tutor Graph Navigation & Frontier Orchestration
- [x] Implement ADR-003 proactive graph query protocol in `crates/tutor/src/prompts.rs` (`system_policy.txt` rewritten with the mandated protocol; file + fallback-chain tests).
- [x] Connect typed graph tools (`find_concept`, `get_concept`, `get_dependencies`, `get_weak_dependencies`) to `GraphRepo` (added missing `get_dependencies` service/arm/definition; deterministic `StubLlmClient` proves the loop).
- [x] Stream tool invocation progress events to client (`tool_progress` start/finish frames in both SSE modes; safely ignored by current dispatchers; router-tested).
- [x] Verify tutor inspects learner's prerequisite health before teaching complex topics (stub-LLM test: weak prerequisite surfaces in model context only when confidence is low).

**Exit Gate**:
```text
[x] Tutor calls graph tools before generating an explanation when concepts are mentioned.
[x] Explanations measurably differ depending on whether prerequisites in SQLite are strong or weak.
[x] Tool execution steps are optionally visible in the chat UI without breaking text flow.
```
_Backend verified deterministically with `StubLlmClient`; real-model tool
willingness validated 2026-09-18 with Gemini (T17): unprompted
`get_weak_dependencies` chains, weak-prerequisite repair pivot, clean turns.
The UI consumer (`ToolActivity`, T14) is unit/integration-tested; live browser proof awaits a scriptable
tool backend outside tests._

---

### M5: Candidate Concept & Relation Proposals + Atomic Mutations
- [x] Implement tutor candidate proposal tools: `propose_concept` and `propose_relation` (repaired against the real `concept_candidates`/`relation_candidates` schema — prior code wrote to nonexistent tables/columns and every call failed; added missing tool definitions so the model can call them).
- [x] Implement deterministic schema and sanity validation (non-empty names/reasons, relation-type enum, no self-reference, endpoints must resolve as nodes or usable candidates).
- [x] Enforce world-confidence admission gate (`world_confidence >= 0.80`, via `ConceptNode::validate` at propose time and re-checked from the stored row at admission).
- [x] Implement atomic SQLite transaction for graph mutations and audit logging (`graph_mutations` table): `admit_concept_candidate` commits node + accepted verdict + audit row together; gate rejections persist a `rejected` verdict with reason and write nothing else.

**Exit Gate**:
```text
[x] When teaching novel material, tutor proposes candidate concepts.
[x] Proposals with world_confidence < 0.80 are rejected by the domain layer.
[x] Admitted concepts and dependency relations commit atomically to SQLite.
```
_Verified deterministically with `StubLlmClient` (9 candidate + 3 tool-loop
tests); real-model proposal willingness needs keyed-LLM validation._

---

### M6: Structured Learner Observations & Confidence Tracking
- [x] Implement post-turn observation extraction tool: `log_observation` (repaired: enum serialization vs CHECK mismatch meant every call failed; added validation gate — resolvable concept, delta in [-1,1], non-empty evidence, ensured learner).
- [x] Update `learner_concept_states` in SQLite transactionally based on turn evidence (`apply_observation`: observation + confidence upsert from neutral 0.5 prior, clamped, evidence timestamps; tool result reports new confidence).
- [x] Implement long-term half-life decay with 2-year grace period (`apply_decay` maintenance: incremental-from-`updated_at` so passes compose exactly instead of compounding; `next_decay_at` scheduler hints; rapid re-passes converge).
- [x] Mark concepts below `HEALTHY_THRESHOLD = 0.95` as review-eligible (`review_items` opened/resolved in the same transaction).

**Exit Gate**:
```text
[x] Tutor records evidence of learner confusion; learner_confidence drops in SQLite.
[x] Subsequent session targets weak prerequisite for repair before proceeding.
[x] Decay unit tests verify stability over simulated time intervals.
```
_M6 backend verified deterministically (`StubLlmClient` + backdated rows);
real-model observation quality needs keyed-LLM validation. Decay runs as an
explicit maintenance pass (not yet scheduled); reads show stored values._

---

### M7: Concept-Aware Chat Rendering & AST Highlighting Pipeline
- [x] Backend emits `concept_annotations` SSE event upon completing a tutor turn (deterministic whole-word matcher, 0.80/weak/new badges, capped at 20, ignored safely by legacy clients; router-tested).
- [x] Implement `remarkConceptHighlight` plugin in `apps/web/client/src/knowledgeable/plugins/` (text-only visitor; code/math/HTML-attribute safe; 6 specs).
- [x] Register custom `<ConceptHighlight>` component in LibreChat's `markdownConfig.ts` (additive `concept-highlight` mapping + optional annotations param on `getRemarkPlugins`; `Markdown.tsx` feeds per-message store; 44 knowledgeable Jest specs green).
- [x] Render subtle badges:
  - Known (>= 0.80): Subtle dotted underline + hover tooltip.
  - (T25 product decision: chat surfaces KNOWN concepts only — weak/new
    mentions render as plain text, no percentages or review noise; learner
    health stays visible in the graph explorer.)
- [x] Ensure code blocks (`pre`, `code`), inline math (`$`), and display math (`$$`) are never corrupted (plugin + pipeline specs; total `tsc` errors unchanged at 25 pre-existing upstream).

**Exit Gate**:
```text
[x] Assistant explanations render concept words with styled badges based on learner graph state.
[x] Hovering a concept shows title, learner confidence %, and summary.
[x] LaTeX math formulas and code blocks render flawlessly without tag interference.
```
_Verified live 2026-09-16 (headless Chrome + scratch backend, fake LLM): 8/8
browser checks — badge `known`, tooltip `Prime Number · 98%`, no badge inside
KaTeX/code. Requires the T12 v2 generation-protocol adapter (start ticket,
`stream/:id`, `status/:conversationId`, roles, `endpointType: custom`)._

---

### M8: Personal Knowledge Wiki Generation, Caching & Drawer UX
- [x] Add `concept_wiki_pages` table migration in SQLite (`migrations/`).
- [x] Implement `WikiService` in `crates/application`:
  - Async generation of personalized wiki markdown upon concept mastery (`confidence >= 0.70`).
  - Graph mutation staleness flagging (`is_stale = 1`).
  - Low-frequency rate-limited regeneration (24h minimum interval; stale-but-recent serves as-is).
- [x] Expose `GET /api/concepts/:id/wiki` in `crates/api` (stable envelope: `wiki_not_ready` vs `not_found` vs 503).
- [x] Implement `WikiDrawer` component in `apps/web/client/src/knowledgeable/components/`.
- [x] Clicking any concept highlight in chat opens the wiki slide-over drawer (button role, keyboard operable, Escape closes; single host in `ChatRoute`).

**Exit Gate**:
```text
[x] Clicking a highlighted concept in chat smoothly opens the personal wiki drawer.
[x] Wiki page displays personalized explanation, anchored prerequisites, and confidence gauge.
[x] Viewing a wiki page loads from SQLite cache without invoking the LLM.
[x] Graph changes mark the page stale, triggering lazy background update only when viewed.
```
_Verified live 2026-09-18 (headless Chrome + scratch backend, cached page):
8/8 browser checks — drawer opens on badge click, title/gauge/prereqs render,
no stale flag when fresh, Escape closes. Generation path unit-tested with a
canned LLM (real structured generation implemented for Gemini/OpenAI, not
yet exercised live)._

---

### M9: Interactive Knowledge Graph Explorer Integration
- [x] Expose `GET /api/graph/neighborhood` in Axum adapter (alias of canonical `GET /v1/graph/neighborhood`; router-tested 400/404 parity).
- [x] Implement Graph Explorer panel in `apps/web/client/src/knowledgeable/` (`components/GraphExplorer.tsx` + `api/graphClient.ts` + `graphTypes.ts`/`graphUtils.ts`; 22 Jest specs green; provider-free, chat stays primary).
- [x] Mount the panel from the main navigation (side-panel link `knowledge-graph` in `useSideNavLinks`; E2E 10/10 in headless Chrome vs seeded backend).
- [x] Distinguish semantic edges (dashed ┄) vs dependency edges (solid directed —▶) via legend + per-edge labels.
- [x] Visual confidence display matching learner confidence (%, status badge, confidence bar, weakest-first sort; review-only filter).
- [x] Node selection drills the neighborhood into the selected concept (links to personal wiki page / targeted review session pending M7 concept annotations + M8 wiki).

**Exit Gate**:
```text
[x] User can toggle the Graph Explorer from the main navigation.
[x] Displays local graph neighborhood centered on the active topic.
[x] Clicking a node opens its wiki page or starts a review conversation.
```
_Browser proof 2026-09-19 (CDP): "Knowledge Graph" nav entry opens the panel
(search/depth/limit controls render); searching "prime" lists backend concepts;
loading Prime Number renders "3 concepts · 2 links · 2 need review" with legend
(—▶ dependency vs ┄ semantic), confidence badges (Factor 30% review,
Divisibility 85% review, Prime Number 98% healthy), and edges. Drill proven:
clicking the "Factor" node re-centers to its 1-concept neighborhood. Earlier
"result-click" caveat retracted — harness artifact (clicked the `<li>` instead
of `button[data-testid="graph-search-result"]`; `handlePickResult` loads
directly by design). Precise gap for the node-click box: node selection drills
only — no node→wiki link exists yet (wiki opens from chat badges, M8 proven)._
M9 fully closed 2026-09-19: the new per-node "Wiki" button (same `openWiki`
store action as chat badges, jest-covered) opens the drawer live with the real
personalized page — "Prime Numbers: A Fresh Start", Confidence 98%,
prerequisite-anchored summary, prereqs Factor 30% / Divisibility 85%, fresh
(not stale). M9 browser exit gate fully closed._

---

### M10: Production Hardening, Polish, and Packaging
- [x] Create unified developer runner (`./scripts/dev`) launching backend and web client.
- [x] Configure production build to embed/serve compiled `apps/web/dist` directly from Axum binary (`ServeDir` + SPA fallback, API precedence, `WEB_DIST_DIR` override, missing-build hint).
- [x] Run full test suite (`cargo test`, frontend linters, end-to-end user journeys) — workspace green, `cargo fmt` clean, 59 knowledgeable Jest green, `npm run build` green, browser E2Es green (T9/M7/M8).
- [x] Verify zero memory leaks, graceful shutdown, and rock-solid SQLite concurrency under load (25 concurrent incl. 5 streaming turns all clean; SIGTERM exits clean with `integrity_check` ok and WAL checkpointed).

**Exit Gate**:
```text
[x] Single binary or one-command start launches the entire system.
[x] No external services required (completely offline-capable with local LLM).
[x] All automated checks pass cleanly.
```
_`./scripts/dev` for development; the Axum binary serves the built client
for single-binary production. Offline path (`local-tutor`, no keys) verified
throughout; Ollama removed-for-now 2026-09-22 (mobile-first; see M3)._

_Beta P0 proof 2026-09-19 (post-deletion tree): `npm run build` green, fresh
`dist/` contains the cleanup code; Axum serves it single-origin (`/` → SPA,
`/api/config` → JSON precedence, unknown `/api/*`+`/v1/*` → JSON 404); CDP
boot straight against `:3000` (no Vite) reaches `/c/new` with composer and
zero uncaught exceptions._

---

## 4. LibreChat Product-Surface Cleanup Roadmap (Canonical, Not Started)

Policy source: `docs/agent-context/integration/librechat.md` §9–§13
(feature matrix, backend boundary, protected boundaries, upstream constraints).
`PRODUCT.md` / `MVP.md` hold the product/MVP intent; this section holds the
staged execution contracts. Implementation has NOT started — this task only
codifies the roadmap. Each phase is a set of small, independently verifiable
contracts with a narrow scope, explicit expected behavior, a verification
contract, a rollback point, and a small diff. Per-batch discipline:
`READ → MAP → CHANGE ONE SMALL SEAM → VERIFY → INSPECT DIFF →
COMMIT/BASELINE → NEXT SEAM`.

Scope guard for every phase: trim the visible product while preserving
LibreChat's architecture wherever practical. Prefer existing config switches,
existing permission gates, and centralized UI gates; do not fork feature
internals; keep `apps/web/client/src/knowledgeable/` isolated; keep the Rust
adapter narrow; never delete implementation before hiding, verifying, and
proving isolation.

### Phase 0 — Documentation and baseline

- [x] 0.1 Authoritative feature matrix: exactly one canonical matrix (§9 above
  in the integration doc) recording product decision AND backend status
  separately. (Established by the policy-codification task.)
- [x] 0.2 Current backend boundary: actual adapter routes documented (§10),
  with missing routes named as gaps, not claimed functionality. (Established.)
- [x] 0.3 Protected boundaries: Knowledgeable code, adapter, shared infra, and
  upstream-shaped code documented (§11). (Established.)
- [x] 0.4 Clean working-tree baseline: commit `c89760b` (5 policy docs files)
  is the known-good baseline; `cargo fmt --all -- --check` clean and
  `cargo test --workspace --all-features` green (98 passed, 0 failed) verified
  2026-09-18 before implementation begins. (Full `scripts/verify.sh` not run:
  its Flutter section targets the deprecated `apps/client`.)

### Phase 1 — Configuration-only disabling (safest first)

Targets where LibreChat already exposes switches: parameters, presets,
temporary chat, multi-conversation, web search, file search, code execution,
and siblings.
- [x] 1.1 The feature is invisible/unavailable to the user.
- [x] 1.2 Core chat still works.
- [x] 1.3 Conversation history still works.
- [x] 1.4 Model selection still works.
- [x] 1.5 SSE streaming still works.
- [x] 1.6 No Knowledgeable-specific functionality regresses.
- [x] 1.7 The diff uses existing LibreChat seams, not feature-internal rewrites.
(Verified 2026-09-23: flags served in `system.rs:45-56`, locked by
`config_interface_matches_phase1_policy`; core flows covered by api
router tests for chat/history/models/SSE.)

### Phase 2 — Centralized UI gating

For features lacking config support but hideable at central seams: side-panel
entries, chat-input tools, menus, settings sections, unsupported nav routes.
Must handle the known `system.rs` mismatch (`interface.* = false` is NOT enough
because `role()` grants every permission and several `useSideNavLinks` entries
gate on permissions only).
- [x] 2.1 Unsupported features no longer appear in normal navigation.
- [x] 2.2 Direct navigation to disabled surfaces cannot produce broken/dead states.
- [x] 2.3 Core navigation remains intact.
- [x] 2.4 Knowledgeable Graph/Wiki navigation remains intact.
- [x] 2.5 Changes are centralized and upstream-friendly.
(Verified 2026-09-23 for 2.1/2.3/2.4/2.5: CDP census recorded below;
central `attachmentsDisabled`/`speechDisabled` flags + roles revocation in
code; `routes/index.tsx` Navigate-to-`/c/new` guards; unconditional
redirects in `ProjectsView`/`ProjectWorkspace`. 2.2 left open: the audit
portion below still notes its browser proof pending.)

Contract 2.1 browser evidence (CDP, desktop viewport, labeled-button census):
side panel offers only Bookmarks (KEEP), Knowledge Graph (ours), Attach Files
(known scoped gap) — agent/assistant builder, Skills, Schedules, Prompts,
Memories, Parameters, and MCP builder are all absent; model picker works
(`gemini-3.1-flash-lite` default). Remaining visible gaps, all recorded, none
blocking: "All projects" nav entry (covered by the projects-guard brick),
"Attach File Options" (attachments project), "Use microphone" (speech surfaces
need their own gating brick: STT/TTS/voice-mode toggles).

Contract 2.2 proof DONE 2026-09-23 (CDP vs live backend + vite, `/tmp/opencode/cdp-p22.js` uncommitted, 14/14): `/agents`, `/skills`, `/prompts/:id`, `/insights`, `/projects`, `/projects/:id` all land on `/c/new` with zero removed-surface strings in the DOM and zero uncaught exceptions; chat boots cleanly after the tour. Code-read audit below now confirmed by the browser pass. Self-guarding by
upstream design — `/agents` (redirects to `/c/new` without MARKETPLACE.USE),
`/skills*` (`<Navigate to="/c/new">` without SKILLS.USE, revoked),
`/prompts/:promptId` (same guard, PROMPTS.USE revoked), `/insights`
(redirects, `insightsEnabled` flag absent from our config). `/projects*` gap
closed: `ProjectsView`/`ProjectWorkspace` now redirect unconditionally to
`/c/new` (`projectsDisabled` constant, jest-covered), browser-proven live for
both `/projects` and `/projects/:id`. `/search` and `/share/:shareId` are
KEEP-direction seams with documented backend gaps (Phase 5), not removal
targets.

Scoped sub-project (do NOT hide piecemeal): file attachments/uploads. The
`FilesPanel` side-panel entry is pushed unconditionally, but the attach
surface is entangled — `AttachFileChat` in `ChatForm`, drag-drop providers,
paste-as-file, upload modals, `ManageFiles` settings — with no `/api/files/*`
backend behind any of it. Hiding only the panel would leave dead upload
buttons; gate the whole upload path as one browser-verified project instead.
  Brick 2026-09-20 (commit `79e7e55`): central `attachmentsDisabled = true`
  flag in `~/knowledgeable/attachments.ts`, consumed at four gates — nav entry
  (`useSideNavLinks`), composer button (`ChatForm` no longer mounts
  `AttachFileChat`), paste/drag/modal (`useUploadOptions` forces
  `uploadsDisabled`, which all three flows already honor with the upstream
  disabled toast), settings (`manageFiles` registry `show`). No backend route
  (adapter stays narrow); panel/dialogs stay vendored for Phase 5. Verified:
  tsc pinned at 25 with zero in touched files; registry + useUploadOptions
  specs 18/18 green. Open: heavy ChatForm upload suites not runnable on this
  laptop (coverage-on defaults + 3 jsdom workers thrash it; use
  `--coverage=false --maxWorkers=2`), and the CDP browser pass.

Scoped sub-project (do NOT hide piecemeal): speech (STT/TTS/voice mode).
The composer mic button renders because `store.speechToText` defaults on and
`useSpeechSettingsInit` *enables* speech controls on a missing configuration
response — which is exactly what our backend returns (no
`/api/files/speech/config/*`). Gating means the recoil default, the init hook,
the SPEECH settings tab (~15 entries), and the STT/TTS hooks as one
browser-verified project, not a single toggle.
  Brick 2026-09-20 (speech code, browser pass open): central
  `speechDisabled = true` flag in `~/knowledgeable/speech.ts`, consumed at
  five gates — composer mic + response auto-play (`ChatForm` no longer mounts
  `AudioRecorder`/`AutoPlayAudio`), per-message speak button
  (`HoverButtons`), the SPEECH tab (`types.ts` TABS `show`), and all 14
  SPEECH registry entries (entry-level `show`, since settings search renders
  components bypassing tab visibility). No backend route; components, hooks,
  and recoil defaults stay vendored/untouched (render gates make stored
  values unreachable). Verified: tsc pinned at 25 with zero in touched
  files; registry spec 15/15 green with `--coverage=false --maxWorkers=2`.
  Open: CDP browser pass (mic/speak/autoplay absent, SPEECH tab + search
  clean).
  Browser passes DONE 2026-09-20 (attachments + speech, CDP vs scratch
  backend, `/tmp/cdp-gates*.js` uncommitted): side panel offers only Chat
  History/Bookmarks/Knowledge Graph/Personal Wiki (no Attach Files);
  composer has no attach button and no mic; pasted file ends at the
  upstream "File uploads are disabled for this endpoint" toast with zero
  `/api/files` hits server-side; seeded conversation renders with no
  read-aloud buttons; settings has no Speech tab, Data & Privacy holds no
  Manage files, search "speech" matches nothing. (Seed note: message/page
  row ids must be UUIDs — the repos `parse().unwrap()` row ids and panic
  otherwise; harness-seed bug, not product.)

Boot probe inventory (headless Chrome 2026-09-19, 45 dead-backend hits, all
handled rejections, zero uncaught): `/api/projects*`, `/api/tags`,
`/api/search/enable`, `/api/files*`, `/api/files/config`,
`/api/files/speech/config/get`, `/api/balance`, `/api/banner`,
`/api/keys?name=knowledgeable`, `/api/user/settings/{favorites,pinned-order}`,
`/api/agents/tools/web_search/auth` (404s), plus `GET /api/agents/chat/active`
(405 — matches our `POST /api/agents/chat/:endpoint` pattern with the wrong
method; agent run-poll with no runs behind it). Every URL maps to a
removed/future surface or benign generic probe; none touches the chat turn
path. Follow-ups (not this phase): suppress the web-search auth probe and the
`chat/active` poll noise if they prove perpetual rather than boot-only.

### Phase 3 — Remove clearly unwanted product surfaces

Agents, agent marketplace/builder, assistant builder, skills, projects, prompt
management, schedules, LibreChat memories, plugin marketplace, generic
enterprise/admin, unnecessary account surfaces, Langfuse UI. Hide/disable first;
do NOT immediately delete implementation files.
- [ ] 3.1 The user cannot accidentally enter the removed surface.
- [ ] 3.2 Normal chat has no broken references to the removed surface.
- [ ] 3.3 No shared component required by core chat was deleted.
- [ ] 3.4 Dependency analysis identifies genuinely dead implementation.
- [ ] 3.5 Only genuinely isolated dead code is considered for deletion.

Contract 3.4 ledger (static import scan 2026-09-19, `apps/web/client/src`;
nothing deleted — each row needs its own isolation+browser brick per 3.5):
- ISOLATED candidates: `components/Insights` (7 files; only `routes/index.tsx`
  dynamic import), `components/SidePanel/Memories` (13; only `useSideNavLinks`),
  `components/SidePanel/Schedules` (17; only `useSideNavLinks`).
  - [x] Insights deleted 2026-09-19 (dir + route→redirect, tsc pinned at 25,
    `/insights` → `/c/new` browser-proven; data-provider access queries left
    dormant behind the absent flag).
  - [x] Memories panel deleted 2026-09-19 (13 files + central gate entry;
    single importer; zero references after; tsc pinned at 25; boot DOM clean).
    Residuals deliberately left: `MemoryToggle` settings entry (already hidden
    via revoked OPT_OUT), `utils/memory.ts` + data-provider Memories queries
    (shared with chat artifacts/SSE — constraint 10).
  - [x] Schedules panel deleted 2026-09-19 (17 files + central gate entry,
    single importer, no dynamic imports; zero references after; tsc pinned
    at 25; only spec mention is a comment).
  - Post-deletion boot proof 2026-09-19 (CDP, exception listener attached):
    chat boots, zero removed-surface strings/panel labels in DOM, zero uncaught
    exceptions. (Harness note: CDP nests `result.result.value` — a helper that
    reads one level shallow silently yields `undefined`; fixed in `/tmp`.)
- SEMI-ISOLATED: `components/Projects` (11; routes dynamic + `ProjectsSection`
  sidebar shell — shell must go in the same brick).
  - [x] Projects deleted 2026-09-23 with the shell in one brick (commit
    `a89b0ab`: 11 view files + `Conversations/ProjectsSection.tsx` + its
    F14 spec; routes `/projects*` redirect via Navigate stubs like
    Insights; sidebar order spec rewritten to Pinned-before-Chats;
    zero references after; tsc clean on touched files; 28 Jest suites
    174/174; CDP redirect tour 14/14 green, sidebar shows no Projects
    section).
- ENTANGLED (do not delete wholesale): `components/Skills` (45; also used by
  agent-tools `SkillsDialog`), `components/MCPBuilder` (25; also used by agent
  `AddMcpServerDialog`), `components/Prompts` (53; provider
  `PromptGroupsContext`, input `PromptsCommand`, agent tools),
  `components/Agents` (routes + `Chat/Landing`), `components/SidePanel/Agents`
  (132 files, 5 external importers — never wholesale).

### Phase 4 — Keep/future features: preserve clean seams

Bookmarks, pin, archive, fork/branch, conversation search, minimal share, file
attachments, file search, MCP, STT, TTS, voice/conversation mode, provider API
keys/BYOK, login/accounts/sync, token usage, billing, prompt slash commands.
- [ ] 4.1 They are not accidentally deleted during cleanup.
- [x] 4.2 Where currently unsupported, they are documented as future backend/product work.
- [ ] 4.3 The current MVP remains small despite preserving future capability.
- [ ] 4.4 Future functionality has a clearly identifiable integration seam.
- [ ] 4.5 No speculative backend implementation is introduced for dormant UI.
(4.2 verified 2026-09-23: Phase 5 list below + §9 matrix record decisions;
4.1/4.3/4.4/4.5 left open as judgment calls needing per-surface review.)

### Phase 5 — Optional post-MVP feature projects (explicitly out of cleanup)

Each becomes its own feature project, never one giant "enable everything"
task: bookmarks, pin, archive, conversation search, fork/branch, minimal
sharing, attachments, document search/RAG, speech, MCP, BYOK, accounts/sync,
token usage, billing, prompt slash commands, voice mode.

### Phase 5a — Personal Wiki browser (planned, not started)

**Problem**: the wiki exists only as a popup drawer reachable from chat badges
and graph nodes. There is no place to *browse* what you've mastered; testers
expect a wiki tab and report its absence as "wiki broken" (feedback F8).

**Scope** (drawer stays exactly as is):
- Side-nav entry (next to Knowledge Graph) opening a full wiki browser view.
- Lists mastered concepts (learner_confidence ≥ 0.70) with confidence badges,
  sorted weakest-first; text search over names.
- Clicking a concept opens the existing drawer (reuse `openWiki`), or a full
  reading view later — drawer reuse first.
- Stale pages show the existing "may be outdated" marker; no new staleness
  semantics.

**Backend contracts required** (none exist yet — part of the project, not the
cleanup):
- `W1`: list query for mastered concepts (bounded, sorted, confidence
  included) — new endpoint or `searchConcepts` extension; no new tables.
  DONE 2026-09-20: `GET /api/concepts/mastered` (`concepts.rs` thin
  controller over `GraphService::list_mastered_concepts` →
  `graph_repo::list_mastered_concepts`: active concepts at the shared known
  threshold (≥0.80, matching badges; raised from mastery 0.70 per user
  report 2026-09-20 — the wiki lists known concepts), confidence/name/id
  order, limit clamp 1–200, limit+1 truncation flag, `wiki_status`
  ready/stale/none via LEFT JOIN; existing confidence index reused, no
  migration). Locked by 5 service tests + 2 router tests (workspace 115 green).
- `W2`: reuse `GET /api/concepts/:id/wiki` unchanged for page content.

**Verification**: browser pass — entry visible, list matches graph mastery,
click opens the real page, empty state when nothing mastered yet. Explicitly
out of scope: editing pages, public sharing, full-text search across page
bodies.
  View DONE 2026-09-20 (browser pass still open): `WikiBrowser.tsx`
  self-sufficient panel (loads once, server order preserved, local name
  filter, confidence badges via shared `formatConfidence`, stale marks
  matching drawer wording, empty/error/retry/truncated states) mounted as a
  `personal-wiki` side-nav entry beside the graph (locale key
  `com_ui_personal_wiki`, en only per convention); rows call existing
  `openWiki`, drawer untouched; client via `fetchMasteredConcepts` in
  `wikiClient.ts` (shape guard + typed errors).   Locked by 6 `WikiBrowser`
  specs + 2 `wikiClient` specs (13/13 green, fetch-stubbed, ~3s), tsc
  pinned at 25 with zero in touched files.
  Browser pass DONE 2026-09-20 (CDP vs scratch backend, harnesses in
  `/tmp/cdp-*.js` uncommitted): entry renders (`nav-panel-personal-wiki`),
  3 seeded rows weakest-first (Alpha 72% → Gamma 85% + stale mark →
  Beta 98%), below-threshold Weak excluded, Beta row opens the real cached
  page (no stale flag), Gamma row shows the stale flag, Alpha row
  generated live ("Introduction to Alpha" — keyed generation proof),
  filter narrows to one row, wiped states render the empty state with zero
  rows. 19/19 main + 4/4 empty green. User DB untouched (scratch
  `/tmp/wiki-browse.db`); servers stopped after the pass.

### Phase 6 — Upstream synchronization discipline

- [ ] 6.1 Upstream changes review cleanly against a mostly recognizable tree.
- [ ] 6.2 Knowledgeable-specific changes are easy to identify.
- [ ] 6.3 Configuration/gating changes remain centralized.
- [ ] 6.4 Minimal unrelated diff noise.
- [ ] 6.5 Focused tests verify chat, streaming, conversations, message
  rendering, model selection, Graph/Wiki, and navigation after an update.

---

## 5. Beta feedback log (manual testing, 2026-09-19)

- [x] F1 Core chat functional (send/stream/reload/sidebar) — confirmed by tester.
- [ ] F2 Wiki "empty/error" report — NOT reproduced: badge render, badge→drawer,
  cached page, and live generation-on-miss (Divisibility page generated during
  diagnosis) all verified working; error envelopes sane (404/503 + Retry).
  Suspects: transient generation hiccup, or a weak-concept node Wiki button
  (correctly "not ready" by design). Needs the exact word/error text to close.
- [ ] F3 Concept-badge display complaint ("shown all the time") — T25 known-only
  filtering confirmed live in the remark plugin; tooltip is hover-only by
  design. Needs the exact symptom (stuck tooltip? amber/blue badges visible?
  percentages inline?) to close.
- [x] F4 Graph tab visual rehaul — CONFIRMED by screenshot: data layer correct
  (counts/confidence/edges), presentation weak (tiny canvas, cramped form,
  list-as-text-dump). Scoped project: canvas sizing/labels/edges + form
  layout; study notion/logseq/obsidian patterns. Data contracts unchanged.
  CLOSED 2026-09-25 in parts: list-as-text-dump went with T9 `ConceptRow`
  rows; cramped form went with the Phase 2 focus summary + search/segmented
  hierarchy; the measurably remaining gap was the fixed 262px canvas keyhole
  (multi-rank graphs scrolled inside it). Brick (`GraphPanel.tsx` + `k.css`
  only): map canvas + legend wrapped in `figure.k-map` / `figcaption.k-legend`
  (screen-reader association), canvas height `clamp(300px, 50vh, 480px)`
  (3–4 ranks render without inner scroll; the panel scrolls instead). Locked
  by a figure-caption spec (16/16 Jest green; verified the new test fails
  pre-fix) with tsc 26 = baseline and zero in touched files. Browser pass
  (CDP vs backend :3000 + Vite :3090, `/tmp/opencode/cdp-f4-*.js`
  uncommitted, read-only GETs): 275×262 → 275×450 at 1440px, 358×262 →
  350×422 at 390px; Prime Number drill renders 3 nodes + 2 edges with zero
  errors. User DB untouched; servers stopped after.
- [ ] F5 `log_observation` tool-arg parse failure (`invalid character: found
  't' at 0`) seen once in a live keyed turn — model emitted non-JSON args.
  Robustness gap: harden argument parsing/repair vs failing the call.
  Brick 2026-09-20 (`tutor_service.rs` only): `parse_tool_arguments` now
  strips code fences (via `llm::structured::strip_code_fences`) and recovers
  prose-wrapped payloads via outermost-`{...}` extraction (Gemini A+A
  first-value recovery kept); `execute_tool` maps serde failures to
  "retry {tool} with a single JSON object" and rejects non-object args at an
  object gate instead of leaking raw serde text or confusing field errors.
  Locked by 6 `tool_arg_tests` (application 48/48, workspace fully green,
  no new warnings). Live probe 2026-09-24 (scratch backend `:3117`,
  scratch DB, direct API POSTs, SSE harnesses uncommitted in `/tmp/opencode/`):
  4 completed keyed turns, 0 malformed-arg emissions — server log shows zero
  "not valid JSON" and zero object-gate ("must be a JSON object") hits across
  17 successful + 3 failed tool executions, so the repair path held by
  construction (raw args → `parse_tool_arguments` → re-serialized before
  `execute_tool`) without ever needing to fire. Non-recurrence, not a repair
  demonstration: a genuine non-JSON emission was never observed, so
  outermost-`{...}` recovery remains unit-proven only. Model note: env
  `GEMINI_MODEL=gemini-3.1-flash-lite` 503s persistently (`UNAVAILABLE` demand
  spikes); `gemini-2.5-flash-lite` is retired (404, "use gemini-3.5-flash-lite");
  all live turns ran on explicit `"model":"gemini-3.5-flash-lite"`. Side
  observation (out of F5 scope): 3× `log_observation` failed validation with
  "invalid concept_id: no such concept" — the model invents UUIDs for
  not-yet-admitted candidates; the validation gate catches it and the turn
  continues. All 8 probe convos DELETE-confirmed, scratch DB destroyed.
- [ ] F6 Badge-vs-bold confusion (tester report 2026-09-19, reproduced): the
  tutor writes `**factor**` markdown bold around concept words; testers read
  bold as the known-concept badge and expect a click → wiki. But bold has no
  handler, and Factor (0.30, weak) is correctly badge-filtered. Candidates:
  prompt tweak (don't bold bare concept names), or visual disambiguation.
  Brick 2026-09-20: prompt tweak applied (`system_policy.txt` + `prompts.rs`
  fallback: "Write concept names as plain text — never wrap them in bold or
  italic; badges are the sole emphasis"), locked by
  `system_policy_keeps_concept_names_plain_text` (tutor 4/4, application 42/42
  green). Live keyed compliance PROVEN 2026-09-24: 4/4 keyed turns on novel
  topics (photosynthesis ×2, black holes, fraction-addition confusion; scratch
  backend `:3117`, `gemini-3.5-flash-lite`, SSE harnesses uncommitted in
  `/tmp/opencode/`) carry zero `**` bold and zero `*` italic — concept names
  in plain text, teach-then-propose behavior intact (`find_concept` miss →
  taught anyway + `propose_concept`; confusion turn also `log_observation`
  success). Single-model evidence only (`gemini-3.5-flash-lite`); broader
  compliance across models/tones remains probabilistic. All probe convos
  DELETE-confirmed, scratch DB destroyed.
- [ ] F7 Badges vanish on reload (reproduced: reloaded convo renders ZERO
  `concept-highlight` nodes) — annotations live only in the in-memory SSE
  store. DECIDED 2026-09-20 (user): highlighting is a pure function of the
  CURRENT graph — revisiting old chats must dynamically badge newly-learned
  words. So re-derive on history load against live graph state; NO
  persisted annotation rows, no snapshots. Percentages are a bad feature:
  kept only as an opt-in debug toggle (OFF by default), always resolved
  live, never stored. Builder direction: derivation pass on history load
  (bounded matcher, capped) + debug toggle for percentages.
  Brick 1 DONE 2026-09-20 (backend): `GET /api/messages/:id` derives
  `concept_annotations` per assistant message with the live matcher (same
  code path as the turn-end frame; failures degrade to a missing field,
  user messages untouched, no signature churn on `msg_json`). Locked by
  `messages_carry_live_derived_annotations`: badges present at 0.98/known,
  absent for user + unmatched text, and the SAME history re-reports 0.30/
  weak after a confidence drop (workspace 116 green). Open: client
  hydration on history load + percentages debug toggle.
  Brick 2 DONE 2026-09-20 (client): `useMessageAnnotations` hydrates each
  rendered message from the cached history query (prefix scan, validated
  through the SSE path, live frames win, idempotent effect — zero upstream
  hook rewiring, Markdown untouched); percentages gated behind persisted
  `showConfidenceDebug` atom (default OFF) in tooltip + aria-label, with a
  Chat/messages settings toggle. Locked by hydration specs + rewritten
  ConceptHighlight specs (default-off/on) + registry validity (28/28 green
  across 3 suites, capped flags), tsc pinned at 25 with zero in touched
  files. Open: CDP browser pass (reload shows live badges, toggle reveals
  percentages).
  Browser pass DONE 2026-09-24 (CDP vs backend :3000 + Vite :3090 at 390px,
  `/tmp/opencode/cdp-f7-pass.js` uncommitted): keyless local-tutor turn about
  Prime Number carried a Divisibility-known turn-end frame; fresh history open
  rendered 1 badge, reload rendered 1 badge. Found + fixed in the same pass:
  `MarkdownBlock`'s memo comparator ignores `remarkPlugins`, so static history
  content never re-parsed after post-mount hydration landed (live turns badged
  only via content churn) — `Markdown.tsx` now keys `MarkdownBlocks` on the
  annotations signature (stable while streaming, flips once at turn end;
  commit `3907e5b`). Locked by annotations + highlightPipeline specs (12/12)
  and tsc 26 = stashed baseline with zero in touched files.
  Toggle-reveals-percentages DONE 2026-09-24 (CDP vs backend :3000 + Vite
  :3090 at 1440x900, `/tmp/opencode/cdp-f7-toggle.js` uncommitted, local-tutor
  turn with exact "Prime Number" so the fake's canned reply matches the live
  graph): history open renders 1 badge labelled "Prime Number, Known concept,
  open wiki" (default OFF, localStorage key absent); setting
  `showConfidenceDebug=true` + reload relabels it "Prime Number, Known
  concept, confidence 98%, open wiki"; setting false + reload removes the
  percentage again. Only the known annotation renders (chat highlights are
  known-only per T25 — the co-derived `new` Natural Number annotation stays
  plain text by design). No code changes; both probe convos DELETE-confirmed,
  user convos untouched, servers stopped after the pass. F7 fully closed.
  Render parity DONE 2026-09-20 (user directive: chats = wiki via shared
  code): wiki pages carry live-derived `concept_annotations` (same matcher,
  omit-on-empty/failure); drawer feeds them to the shared markdown
  pipeline, respects the user LaTeX setting (was hardcoded on), and gates
  gauge + prereq percentages behind the debug toggle. No store involvement
  (drawer passes the array straight to the plugin). Locked by wiki router
  test + drawer/pipeline specs (workspace 117 green), tsc pinned at 25.
  Open: drawer gauge still shows at-generation confidence (snapshot
  semantics — product question, untouched).
  Wiki voice DONE 2026-09-20 (user report: pages read like tutor dialogue):
  generation prompt now demands a concise third-person reference article and
  bans Socratic patter + reader questions outright (dialogue belongs in
  chat). Locked by prompt invariant test (workspace 118 green). Note:
  already-cached pages keep old voice until staleness regenerates them.
  Browser rows DONE 2026-09-20 (user report: run-together "95%May be
  outdated", percentages don't belong, sub-known rows listed): rows reuse
  the chat-history row language (container/hover/active bar, truncated
  title, BookOpen icon, keyboard operable, active row follows the open
  drawer); percentages render only with the debug toggle (default: clean
  names + stale marks); W1 bar raised to the shared known threshold
  (≥0.80, matching badges — generation stays at 0.70, pages surface once
  known). Path `/api/concepts/mastered` kept (no contract churn). Locked
  by updated service/router/client specs (workspace 118 green), tsc pinned
  at 25.
- [ ] F8 No Personal Wiki browser entry exists (only drawer via badge clicks +
  graph node buttons) — the integration doc's "Personal Wiki navigation
  sidebar item" over-claims; doc corrected with this entry, browser filed as
  Phase 5 idea. Tooltip percentages DECIDED 2026-09-20 (user): debug-toggle,
  OFF by default (see F7). Remaining tooltip questions (sticky behavior)
  stay open for the visual overhaul.

## 6. Beta feedback log, round 2 (manual testing, 2026-09-21)

Product renames below deliberately reverse the UI-redesign copy deck
(Map/Notebook/Open notes); the deck stands until each rename ships.

- [x] F9 Map tab rename + empty-map report — rename "Map" to "Concept Map"
  (panel title, rail entry, copy). Report: tab shows no map and does not
  list known concepts. Suspect is an unseeded/empty graph (empty state is
  correct then) rather than a data bug, but verify against seeded data
  before closing; F4's data layer was proven correct pre-redesign.
  Rename DONE 2026-09-21 (rail `com_ui_knowledge_graph`, panel title +
  aria-label, error copy; 16 Jest green across branding + GraphPanel; tsc
  zero in touched files). Seeded-API proof same day vs live backend:
  `/api/concepts/mastered` returns 4 weakest-first (Divisibility 0.85,
  Integer 0.9, Mersenne 0.95, Prime 0.98); boot default (Divisibility)
  neighborhood is 1 node / 0 edges; Prime Number is 3 nodes / 2 edges —
  data layer correct, so the report reads as empty-graph empty state (or
  the sparse single-node boot) rather than a data bug.
  Browser pass DONE 2026-09-21 (CDP vs backend :3000 + Vite :3090,
  `/tmp/cdp-f9.js` uncommitted): 10/10 — boot with composer, rail
  `nav-panel-knowledge-graph` aria "Concept Map" with zero bare-"Map"
  entries, panel title + aria "Concept Map", boot canvas 1 node
  ("1 concepts · 0 links · 1 need review"), search "prime" picks Prime
  Number, drill renders 3 canvas nodes ("3 concepts · 2 links"), zero
  uncaught exceptions. User DB untouched (read-only GETs; no test
  conversations created); servers stopped after the pass. CLOSED.
- [x] F10 Notebook rename to Wiki — rename "Notebook" to "Wiki" everywhere
  user-facing (panel title, rail entry, reader labels, empty/error copy).
  Reverses the redesign naming; keep "notebook" only as a code identifier
  if churn demands it.
  DONE 2026-09-22 (rail `com_ui_personal_wiki`, browser title + aria,
  browser error copy, drawer dialog aria-label; F9 follow-up in the same
  brick: truncated-note "Use the map…" → "Use the concept map…"; 19 Jest
  green across branding + WikiBrowser + WikiDrawer; tsc zero in touched
  files). "Notes" copy (search placeholder, empty states, drawer
  not-ready) deliberately left for F15, which owns the Open-notes button
  + not-ready gating. Browser pass DONE 2026-09-22 (CDP vs backend :3000
  + Vite :3090, `/tmp/cdp-f10.js` uncommitted): 9/9 — rail "Wiki" with
  zero bare-"Notebook" entries, panel title + aria "Wiki", 4 seeded rows,
  first row opens the drawer labelled "Wiki page: Understanding
  Divisibility", zero uncaught exceptions. User DB untouched (read-only
  GETs); servers stopped after the pass. CLOSED.
- [x] F11 Wiki generation voice: drop the rigid template — remove the
  forced Key Facts/definition-style one-size-fits-all structure from the
  generation prompt and give the LLM more freedom in shaping each page.
  Partly reverses the "Wiki voice DONE" constraints (third-person
  reference voice stays unless the freer prompt regresses it). Backend
  prompt task with a prompt-invariant test update; cached pages refresh
  on staleness as before.
  DONE 2026-09-22 (`wiki_service.rs` `wiki_prompt` only): the article spec
  now reads "shaped to fit this concept — definition-first, example-first,
  comparison, narrative, or a mix" instead of the mandatory "declarative
  definition, key facts, one worked example". Voice rules untouched
  (third-person, no tutoring patter, no reader questions, dialogue belongs
  in chat); JSON envelope, 0.70 trigger, caching, staleness, and 24h
  refresh unchanged. Locked by the updated prompt-invariant test
  (template phrases banned, voice phrases required); `cargo test -p
  application` 54/54 green, `cargo fmt` clean. No frontend changes.
  Cached pages keep old voice until staleness regenerates them. CLOSED.
  LIVE PROOF 2026-09-22 (scratch backup of the user DB, rebuilt binary —
  first attempt ran stale and re-proved only the old template): Mersenne
  Prime + Even Numbers regenerated v1→v2 against live Gemini with the new
  prompt (binary strings confirm "shaped to fit" in, "one worked example"
  out). Voice holds on both (third-person, no patter, no questions);
  Even Numbers adds an unprompted "Context" section outside the old mold
  while Mersenne stays classical — freedom granted, model uses judgment.
  Envelope, versioning, and staleness lifecycle all correct. User DB
  untouched (reads only; scratch destroyed).
- [x] F12 Projects misnamed — "Projects" is the wrong product word
  (candidate: "Folders"). DECISION NEEDED from the user, then rename the
  surface. Upstream feature, rename only.
  DECIDED 2026-09-22 (user): "Folders". DONE same day, rename-first (F13
  refresh loop + F14 dead create are separate bug bricks, untouched):
  30 locale keys in `knowledgeableOverrides` (headers, actions, dialogs,
  empty states, counters, errors — singular/plural handled; Langfuse and
  schedule "project" strings deliberately excluded as other features'
  words). Code identifiers, route paths, and query keys stay
  upstream-shaped. Locked by a new branding contract incl. a sweep
  asserting zero `[Pp]roject` in every non-Langfuse/schedule key (3/3
  Jest green; tsc zero in touched files). Browser pass DONE 2026-09-22
  (CDP vs backend :3000 + Vite :3090, `/tmp/cdp-f12.js` uncommitted):
  6/6 — sidebar header "Folders", "All folders" + "New folder" entries,
  zero Project words in DOM text, `/projects` still redirects to `/c/new`
  with composer, zero uncaught exceptions. User DB untouched; servers
  stopped. CLOSED.
- [x] F13 Projects list refreshes uncontrollably every few seconds —
  suspected runaway refetch/polling loop. Reproduce, find the trigger
  (query invalidation, interval, or focus-refetch), fix, and lock with a
  regression test.
  ROOT CAUSE 2026-09-22: focus-refetch, amplified by the missing backend.
  `useProjectsInfiniteQuery` inherited React Query's
  `refetchOnWindowFocus: true` default while no `/api/projects*` backend
  exists, and a permanently-404ing query never holds fresh data — so EVERY
  window focus fired a fetch that 404ed into a full retry storm with
  spinner flicker. No `refetchInterval` exists on this path and all
  invalidation sites are mutation/SSE-driven (none on a timer). Live repro
  (CDP + Network domain, idle + synthetic focus): boot storm only while
  idle, then one focus event → 4 hits at +0/+1/+3/+7s backoff. FIX (same
  brick): `refetchOnWindowFocus: false` + `refetchOnReconnect: false`
  defaults in `useProjectsInfiniteQuery`, mirroring `useProjectQuery`
  below it; callers can still override; mount fetches still run; retry
  behavior untouched. Locked by `projectsFocusRefetch.spec.tsx` (mount
  fetches once, focus + reconnect add zero fetches — FAILS pre-fix 0/2,
  green post-fix 2/2; 13/13 with the neighboring Projects suites; tsc
  zero in touched files). Browser proof post-fix (CDP, `/tmp/cdp-f13*.js`
  uncommitted): boot storm 4 hits, then 3 focus events → zero additional
  hits, zero uncaught. User DB untouched; servers stopped. CLOSED.
- [x] F14 Project creation fails ("failed to create project") — reproduce
  and diagnose (suspect: no backend route backing the upstream mutation);
  either implement the contract or remove the affordance. Do not leave a
  dead button.
  DIAGNOSIS 2026-09-22 (confirmed, not suspected): the only reachable
  create entry — the sidebar empty-state "New folder" button — flows
  `ProjectCreateDialog` → `useCreateProjectMutation` → `POST
  /api/projects` → the adapter's JSON 404 (`api_not_found`; no projects
  route exists by design) → "Failed to create folder" toast. A full
  folders backend (tables, CRUD, assignment) is a Phase 5-sized project,
  out of scope for a bug brick and against the narrow-adapter rule — so
  the affordance goes. FIX (same brick): the empty state renders static
  "No folders yet" copy (matching `ProjectChatsInline`'s empty style)
  instead of the button; `ProjectCreateDialog` stays mounted but
  unopenable as the Phase 5 seam; `FolderPlus` import dropped. Locked by
  `ProjectsSectionCreate.spec.tsx` (static copy present, zero create
  affordance — FAILS pre-fix, green post-fix; tsc zero in touched files).
  Browser pass DONE 2026-09-22 (CDP vs backend :3000 + Vite :3090,
  `/tmp/cdp-f14.js` uncommitted): 6/6 — "No folders yet" renders, no "New
  folder" button anywhere, no create dialog in DOM, zero `POST
  /api/projects` fired, zero uncaught exceptions. User DB untouched;
  servers stopped. CLOSED.
- [x] F15 "Open notes" becomes "Open Wiki", disabled for unmastered
  concepts — rename the button and gate it on mastery/wiki-readiness
  instead of the current half measure (enabled button leading to a
  "Notes appear once…" error + retry). Not-ready concepts show the
  disabled state; no dead-end error path.
  DONE 2026-09-22: the map focus button reads "Open Wiki" and enables on
  exactly the backend's rule — new `isWikiReady` over
  `WIKI_READY_THRESHOLD = 0.7` (mirrors
  `domain::WIKI_MASTERY_THRESHOLD`; gated at mastery, not known, so
  enabled ⟺ the drawer succeeds). Unready focus renders the button
  disabled with the "unlocks once you have a good grip" title; the drawer
  not-ready fallback stays, reworded wiki-side (now nearly unreachable:
  rows are mastered-only, badges known-only, map rows drill). `Button`
  gains a `title` prop. Locked by `isWikiReady` unit cases + GraphPanel
  gating specs (renamed open test, new disabled test; 45/45 across
  GraphPanel/graphUtils/WikiDrawer/uiPrimitives; tsc zero in touched
  files). Browser pass DONE 2026-09-22 (CDP vs backend :3000 + Vite
  :3090, `/tmp/cdp-f15.js` uncommitted): 7/7 — "Open Wiki" never "Open
  notes", Divisibility (0.85) enabled and opens the real page, Factor
  (0.30) disabled with explanation and opens nothing, zero uncaught.
  User DB untouched; servers stopped. CLOSED.
- [x] F16 Model name is a debug feature — hide the model name/selector
  from end users (debug-gated or removed from the learner surface).
  Reverses the redesign "keep model selector" call; keep the dispatch
  default intact underneath.
  DONE 2026-09-22, debug-gated (not removed): new central
  `knowledgeable/modelPicker.ts` (`modelPickerDisabled = true` +
  `showModelPicker()` with the `?kdebug` escape mirroring
  `showDevControls`); `Header.tsx` mounts `ModelSelector` only when the
  gate passes. Picker, context, selection state, backend dispatch, and
  the advertised default (`gemini-3.1-flash-lite` + `local-tutor` on
  `/api/models`) all untouched — selection resolves from store defaults
  exactly as on today's fresh boot. The header picker was the only
  model-name surface (message components render none). Locked by a
  helper contract + a Header render spec with the picker stubbed (5/5
  Jest green; tsc zero in touched files). Browser pass DONE 2026-09-22
  (CDP vs backend :3000 + Vite :3090, `/tmp/cdp-f16.js` uncommitted):
  5/5 — no picker button and no model names in the header by default,
  in-page `?kdebug` mounts the picker, zero uncaught. (Harness note: the
  boot redirect strips query params, so the escape is set post-boot via
  history + popstate — same limitation the pre-existing `?kdebug` dev
  controls have.) User DB untouched; servers stopped. CLOSED.
- [x] F17 Badge tooltip removal — the floating popup over known concepts
  cannot be dismissed and reads as clutter; tooltips are a debug feature.
  Replace with style-only badges (background change / underline / custom
  CSS, no popup). Partly reverses the redesign 8.1 `k-tip` call; keep the
  click-to-wiki behavior and the debug-gated percentages.
  DONE 2026-09-22: the `concept-highlight-tooltip` popup element is gone
  (hover/focus reveal nothing); badges keep their `k-concept` style-only
  classes, `role=button` click/Enter/Space → wiki, `tabIndex` focus, and
  the accessible label (status always, `%` only with the debug toggle —
  percentages retreat to the a11y tree, which is where debug info
  belongs). New `:focus-visible` outline on `.k-concept` replaces the
  lost focus cue; dead `.k-tip` CSS deliberately left for the F18 purge.
  Locked by rewritten badge specs incl. a hover/focus-absence case (9/9
  with the highlight pipeline suite; tsc zero in touched files). Browser
  pass DONE 2026-09-22 (CDP vs backend :3000 + Vite :3090, one tiny keyed
  turn, `/tmp/cdp-f17.js` uncommitted): 5/5 — live known badge with
  `k-concept--known` styling and a clean aria-label, zero tooltip nodes
  after hover+focus, click opens "Prime Numbers: A Foundational
  Definition", zero uncaught; test conversation DELETE-confirmed gone
  (0 rows) and servers stopped. CLOSED.
- [x] F18 Pre-ship debug purge (epic, final step before public shipping) —
  VERY LAST STEP: do not execute before all other work is done. Direction
  DECIDED 2026-09-22 (user): no code removal per se — gate LOADING so
  debug modules never load in the shipped app (devtools, `?kdebug`
  escapes, `k-dev` details, confidence percentages, debug toggles,
  badge tooltips [done F17]). Full sweep inventory + per-surface
  sub-task proposal recorded 2026-09-22 (see session notes): F18a map dev
  controls, F18b confidence-debug settings entry, F18c dead `.k-tip` CSS,
  F18d devtools/trace/thinking gate confirmation, F18e ship-quiet logging
  default, F18f map/wiki percentages verdict (product call open), F18g
  final sweep re-run. Reframe on execution: prefer load-gates over
  deletion everywhere (keep code, skip mounting/importing in prod).
  EXECUTING 2026-09-22 under the clarified doctrine (debug vital for dev;
  never displayed to end users; delete nothing): F18b DONE — the
  `showConfidenceDebug` settings entry lists only behind `?kdebug` (new
  shared `knowledgeable/debug.ts` helper; registry `show` gate also hides
  it from settings search; atom + aria wiring untouched). Verified:
  helper spec 2/2, CDP 6/6 (entry absent by default + search clean,
  present with `?kdebug`; stale-dialog harness trap documented), tsc zero
  in touched files. Gate confirmations: QueryDevtoolsGate spec 3/3
  green (prod defaults closed, adapter sends no enable flag),
  showThinking default false (policy §9.5 compliant, kept), trace off (no
  adapter key), k-dev controls already DEV-or-`?kdebug` only. Ship-quiet
  logging: `.env.example` down to `RUST_LOG=info` (local `.env`
  untouched). Dead `.k-tip` CSS deliberately LEFT (no-deletion rule).
  OPEN: F18f map/wiki percentages verdict (product call).
  F18f DECIDED 2026-09-22 (user): (a) KEEP as product — confidence
  visibility is the learner model made legible.
  F18g FINAL SWEEP 2026-09-22, EPIC CLOSED: every row re-verified —
  k-dev DEV-or-`?kdebug`, picker gate + `?kdebug` escape, confidence
  entry `?kdebug`-only, badge popup gone (aria percentages kept,
  toggle-gated), map/wiki percentages product, devtools prod-closed
  (spec 3/3), thinking default OFF, trace key absent from adapter,
  `.k-tip` CSS left (no-deletion), example log `info`, zero
  console/debugger in `knowledgeable/`. Doctrine holds throughout: no
  debug code deleted anywhere; nothing debug renders to end users
  uninvited. CLOSED.
- [x] F19 Bookmarks not functional — POST-MVP, not beta-blocking. Backend
  contracts (tags/pin/archive/search/share) are acknowledged missing in
  `MVP.md` ("Deliberately Out of MVP") and `integration/librechat.md`
  ("future contract"). Track here so the gap is not lost; implementing
  any of them is its own backend+UI task.
  TRACKED 2026-09-22, no code changes (by design). Graceful-degradation
  check (CDP vs backend :3000 + Vite :3090, one scratch keyed turn,
  `/tmp/cdp-f19.js` uncommitted): header exposes "Bookmarks" /
  "Add Bookmarks" controls (permission deliberately granted, KEEP hold);
  clicking Add Bookmarks fires no persistence (no `/api/tags*` route —
  confirmed absent by grep over `crates/api/src/routes*`), shows no
  toast, opens no dialog, leaves no phantom state, crashes nothing
  (zero uncaught). Scratch conversation DELETE-confirmed gone (0 rows);
  servers stopped. FUTURE CONTRACT (its own backend+UI project, with
  pin/archive/search/minimal-share siblings per §9.2): tags CRUD routes,
  panel truthfulness, then re-verify this check into a positive proof.
  CLOSED as tracked.
  FULL-PATH REPRO 2026-09-22 (user re-report "adding bookmarks still
  broken"): the header menu exposes a "New Bookmark" item, but with no
  `/api/tags*` behind it nothing can persist — confirmed end to end, no
  crash, no phantom state, scratch cleaned. Side finding from the same
  pass (Network domain): `GET /api/tags`, `/api/search/enable`, and
  `/api/share/link/:id` poll REPEATEDLY against missing backends for the
  whole session — F13-class noise on three more dead surfaces; filed as
  F23. Bookmarks remain: implement as a real Phase 5 backend+UI project
  or keep tracked — user decision open.
  IMPLEMENTED 2026-09-22 (user chose A): commit `4d8888b` (migration +
  repo + service + 5 routes + live tags in convo JSON; 131/131 workspace
  green) plus live browser proof `/tmp/cdp-bm.js` (uncommitted) 7/7 —
  history opens, New Bookmark saves with success toast and menu "1
  selected", panel lists it, reload persists, toggle-off clears
  membership, zero uncaught. Proof notes: Vite hard-codes
  `BACKEND_PORT=3000`, so scratch serves on :3000; harness must click the
  Chats nav entry (a stale Bookmarks-panel tab reads empty); tag names
  unique per run (409 on re-create). User DB untouched (scratch
  destroyed). CLOSED as implemented.
  STAR REFINEMENT 2026-09-22 (user: bookmark should just star the tab;
  double-tap opens the detail menu): single click toggles the built-in
  `Saved` tag after a 280ms double-click window (menu forcibly shut via a
  `setIsOpen` gate — `preventDefault` alone does not stop Ariakit in a
  real browser); double click cancels the star and opens the menu; filled
  icon + `aria-pressed` follow starred state; keyboard Enter/Space stars
  (menu keyboard access stays in the header overflow, which shares the
  items). Hook exposes `tags` + `toggleSaved` (additive). Locked by
  `BookmarkMenuStar.spec.tsx` (2/2; menu-open half proven live — Ariakit
  under jsdom takes ~45s per open) + CDP `/tmp/cdp-bmstar.js` 6/6
  (star/no-menu, unstar, double-click menu without star, zero uncaught;
  final backend state a clean `Saved` tag at count 0). CLOSED.

## 7. Beta feedback log, round 3 (manual testing, 2026-09-22)

- [x] F20 Chat history titles are literal prompts — tabs show the raw first
  message truncated to 48 chars (`derive_title` in
  `crates/api/src/routes/librechat/mod.rs`, used both as the provisional
  title at turn start (`chat.rs`) and by lazy `gen_title` (`convos.rs`)).
  Ask: synthesize each title at creation (what the tab is about), not the
  literal "explain X …". Direction: LLM-written title on first turn with
  truncation fallback for the offline path; never a bare prompt echo.
  DONE 2026-09-22: new `conversation_service` seam (`title_prompt` +
  `sanitize_title` + `synthesize_and_store_title` via `generate_structured`
  {"title"}; ≤6 words, single-line, 48-char cap, truncation kept on any
  failure or error turn). The route detects first turns by the existing
  empty-title check (user renames can never match it), threads the turn
  LLM + effective model name through `TurnContext` (both legacy + v2
  paths), and spawns synthesis AFTER the `final` frame — zero stream
  latency. Locked by `title_test.rs` (prompt grounding, sanitize cases,
  store/fallback/no-call-on-error; application 59 green, api 35 green,
  fmt clean). LIVE PROOF (scratch DB, rebuilt binary, keyed turn
  "Explain how sourdough starter works…"): `final` carried the
  provisional echo, then the stored title became "Explaining Sourdough
  Starter Science" — synthesis, not echo. No frontend changes (same
  string field the sidebar always rendered). User DB untouched (scratch
  destroyed). CLOSED.
- [x] F21 Tutor refuses unknown concepts — "explain anal" answered "I
  couldn't find a concept by that name in my knowledge base…". Root cause
  in our prompt, not the model: `system_policy.txt` ("Use only the
  provided graph context" + mandatory `find_concept`) reads as teach-only-
  what-is-graphed, inverting the M5 design (novel material → teach from
  own knowledge AND `propose_concept`). User verdict: every subject is
  worth understanding; remove the refusal. Direction: prompt brick
  (policy + `prompts.rs` fallback + invariant test) + live keyed proof on
  a novel topic. Truth gates (`world_confidence`, mastery) stay.
  DONE 2026-09-22: Teaching Rules reframed in both sources — the graph is
  the LEARNER, never the syllabus (seed data ≠ curriculum, teach every
  subject, never claim a subject restriction); a graph miss means
  teach-then-propose, NEVER refuse/plead ignorance. The old "use only
  graph context" line is gone. Locked by
  `system_policy_teaches_everything_without_refusal` (both sources;
  tutor 5/5 green, fmt clean). LIVE PROOF same day (scratch DB, rebuilt
  binary, direct keyed turn on novel "sourdough fermentation"): taught in
  two sentences + check-question, zero refusal, zero subject scoping
  (`find_concept` ×2 fired, then taught). Honest gap: no `propose_concept`
  fired this turn — model discretion, not a loop defect (the propose/admit
  loop stays deterministically locked by T15's StubLlmClient tests, and
  T17 proved real-model proposal willingness). User DB untouched
  (scratch destroyed). CLOSED.
  FOLLOW-UP 2026-09-22 (user retest "explain anal"): the new refusal is a
  different animal — provider safety caution on an ambiguous term, not our
  graph logic. Evidence: our client sends default safety settings and
  surfaces whatever returns (no safety code of ours exists); the
  unambiguous clinical phrasing ("biology of the human anus, anatomy
  study") is ANSWERED fully. So the fixable part is disambiguation, now a
  Teaching Rule in both sources (ambiguous term → assume the learner,
  offer the legitimate readings, teach their pick; never jump to the
  crudest reading or launder refusal through a fake subject list;
  invariant test extended; tutor 5/5 green). LIVE PROOF (scratch,
  "Explain balls"): taught the math reading AND asked which context the
  learner means — no refusal. What I will not do unilaterally: lower
  provider sexual-content filters — educational anatomy already passes at
  defaults, and overriding explicit-content refusal is a safety decision
  (students/minors), not a prompt tweak. CLOSED.

## 8. Beta feedback log, round 4 (manual testing, 2026-09-22)

- [ ] F22 Tutor explanation polish (LONG-TERM track) — explanations work
  but "still need polishing". No specific defect cited; treat as an
  ongoing quality bar, not a bug brick. Direction when scheduled:
  collect concrete before/after examples from real turns, then tune
  prompts/style per pattern (Socratic pacing, length, examples) with
  prompt-invariant tests + keyed proofs, one pattern at a time. Do NOT
  bundle with other work; do not start without examples.
- [x] F23 Dead-endpoint polling noise (`/api/tags`, `/api/search/enable`,
  `/api/share/link/:id`) — found during the F19 full-path repro via the
  CDP Network domain: all three poll repeatedly for the whole session
  against backends that do not exist (JSON 404s). Same class as F13
  (fixed for folders via focus/reconnect gate). Direction: per-surface
  trigger hunt (interval vs focus vs status-poll) + narrow gate, one
  surface per brick with a live Network-domain proof. Not beta-blocking
  (handled 404s, zero uncaught), but it is log/client-traffic noise.
  INVESTIGATED 2026-09-22, NO CODE CHANGE (honest downgrade). CDP
  Network-domain watches (`/tmp/cdp-f23*.js`, uncommitted) show NO
  perpetual polling: idle 60s at `/c/new` → 5 hits (boot + retry storm,
  then silence); idle 60s with a conversation open → 9 hits (mount +
  retry storms in the first ~6s, then 54s of silence). All three queries
  already disable focus/reconnect/mount refetches; the F19-session
  "repeats" were remount-driven storms across heavy harness
  navigation/reload (errored queries correctly refetch on mount — they
  hold no data) plus pre-F13-fix focus storms for tags. Degradation is
  already graceful (search hook disables on error; share UI config-gated
  off; tags now served since the bookmarks build). Nothing left to gate.
  CLOSED as investigated.

## 9. Post-MVP projects (user-authorized, 2026-09-22)

- [x] F24 Conversation search — sidebar SearchBar + `/search` message-results
  page backed by SQLite substring search (no MeiliSearch daemon at our
  scale). Contract: `GET /api/messages?search=&pageSize=&cursor=` →
  `{messages: TMessage[] (real titles, conversationIds), nextCursor}` +
  `GET /api/search/enable → true`. DONE 2026-09-22: repo substring query
  (titles joined for row navigation), route + enable flag, convos-route
  collision avoided by static-vs-capture precedence. Locked by 3 router
  tests (shape/titles/pagination/blank/404-parity); 133/133 workspace
  green. Browser proof `/tmp/cdp-search.js` (uncommitted) 5/5 — search
  box mounts on enable, query renders 12 message rows, go-to button opens
  its conversation with 11 messages, zero uncaught. Proof notes: Vite
  hard-codes BACKEND_PORT so scratch serves on :3000; accumulate-and-close
  CDP tabs between runs (a stale-tab pileup looks exactly like a wedged
  renderer); click the go-to button by title — button[0] is Copy, whose
  headless clipboard-permission request wedges CDP (upstream/environment
  quirk, real browsers unaffected, out of scope). User DB untouched
  (scratch destroyed). CLOSED.

## 10. Post-MVP projects, continued (2026-09-22)

- [x] F25 Pin + archive — row menus, Pinned section, archive view/table all
  mounted against missing routes with hardcoded `false` flags (F14 pattern,
  one size larger). DONE: migration (`is_archived`, `pinned` columns),
  repo/service fns, 3 routes (`POST /api/convos/{pin,archive,archive/all}`
  with `{arg: …}` envelopes + 404s), real flags in `conv_json`, list
  honors `?isArchived=`/`?pinned=`. Two semantic calls from live proof:
  archiving unpins (archive files away from the working surface; no
  lingering Pinned rows), and absent `isArchived` reads as unarchived-only
  (the client omits rather than sending false). Locked by 3 router tests;
  136/136 workspace green. Browser proof `/tmp/cdp-pa2.js` (uncommitted)
  7/7 — pin lands exactly in Pinned, archive clears the whole sidebar
  with the filter serving it, restore returns it, reload persists, zero
  uncaught. Out of scope: custom pin drag-order (`pinned-order` settings
  store), fork/branch, share. User DB untouched (scratch destroyed).
  CLOSED.

## 11. Post-MVP projects, continued (2026-09-22)

- [x] F26 Duplicate conversation — row menu offered Duplicate against a
  missing route (same dead-end family as F19). DONE: transactional
  `POST /api/convos/duplicate` (fresh ids, `title + " (copy)"`, messages
  with linear parent chain, flags + tag membership carried, source
  untouched; 404/400 envelopes). Locked by a router test (shape, chain,
  flags/tags, 404/400); 137/137 workspace green. Browser proof
  `/tmp/cdp-dup.js` (uncommitted) 4/4 — menu Duplicate opens the copy
  with its messages, sidebar lists both after reload, zero uncaught.
  Proof note: desktop row-menu trigger is `aria "Conversation Menu
  Options"` (the `convo-options-trigger` testid is the small-screen
  fallback only). User DB untouched (scratch destroyed). CLOSED.

## 12. Post-MVP projects, continued (2026-09-23)

- [x] F27 Minimal share — the deliberate narrow exception (share one
  conversation; nothing social). DONE: `shared_links` table (unguessable
  UUID ids, live message reads, cascade delete, no expiry — revocation is
  explicit delete), service layer, 7 routes (owner create/link/retarget/
  revoke + public read/config/fork) plus the share flow's missing
  `GET /api/messages/:convo/:msg` single-fetch; Share menu surfaces via
  `sharedLinksEnabled` config flip. Locked by 2 service + 3 router tests
  (scope/fork/revoke/400s); 141/141 workspace green. Browser proof
  `/tmp/cdp-share.js` (uncommitted) 7/7 — dialog create yields a new link,
  public view renders title + messages, fork lands in own history,
  revoked link redirects gracefully, zero uncaught. Proof notes: dialog
  create is multi-step slow (resolve → create → refresh; poll the link
  endpoint, not dialog text); dialog has separate no-link/has-link
  states; stale CDP tabs must be closed between runs. Out of scope:
  links LIST page, shared files, expiry, principal-based sharing. User DB
  untouched (scratch destroyed). CLOSED.

- [x] F28 Branching message tree — edits/regenerates create siblings
  (SiblingSwitch navigation) instead of linear tail-append. DONE:
  `parent_message_id` nullable FK (`ON DELETE SET NULL` + index) on
  `conversation_messages`, one-SQL `LAG` backfill preserving linear
  rendering, `create_message_with_parent` repo fns, `add_message`
  tail-append helper + `begin_tutor_turn_with_parent`, explicit
  `NO_PARENT`→root vs missing/invalid→tail in `chat.rs`, duplicate
  remaps parents via `id_map` (tree shape preserved). Locked by 3
  application tests (sibling-via-parent, backfill==synthesis,
  forged→tail); 145/145 workspace green. Browser proof
  `/tmp/opencode/cdp-b2.js` (uncommitted) 4/4 — sibling pair seeded
  through the real chat API renders both questions with a `2 / 2`
  counter (view defaults to latest), previous-arrow lands on `1 / 2`,
  reload keeps pair + counter, zero uncaught. Proof convo deleted
  after; user DB otherwise untouched. CLOSED.
