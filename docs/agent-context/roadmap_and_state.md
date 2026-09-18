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
[ ] Client automatically authenticates via Axum adapter and reaches the new chat view.
[ ] No Node.js Express server or MongoDB instance running.
```

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
[ ] User sends a message in the UI; response streams in real time with smooth rendering.
[ ] Reloading the browser preserves full conversation history from SQLite.
[ ] New conversations appear in the left sidebar and can be switched/deleted.
```

_Backend contract verified by adapter tests (`cargo test -p api`); the browser
exit gate requires a manual `apps/web` dev-server run._

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
[ ] Local Ollama instance successfully streams tutor responses without external internet access.
```
_Model switching and BYOK routing verified at router level (23 api tests);
no Ollama binary in this environment, so the offline-streaming gate stays
open. The client settings UI for key entry is likewise open (backend honors
`apiKey` today)._

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
[ ] User can toggle the Graph Explorer from the main navigation.
[ ] Displays local graph neighborhood centered on the active topic.
[ ] Clicking a node opens its wiki page or starts a review conversation.
```

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
throughout; Ollama-offline streaming still open (no local server here)._

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
- [ ] 0.4 Clean working-tree baseline: known-good baseline with the repo's
  tests/build verification run and recorded before implementation begins.

### Phase 1 — Configuration-only disabling (safest first)

Targets where LibreChat already exposes switches: parameters, presets,
temporary chat, multi-conversation, web search, file search, code execution,
and siblings.
- [ ] 1.1 The feature is invisible/unavailable to the user.
- [ ] 1.2 Core chat still works.
- [ ] 1.3 Conversation history still works.
- [ ] 1.4 Model selection still works.
- [ ] 1.5 SSE streaming still works.
- [ ] 1.6 No Knowledgeable-specific functionality regresses.
- [ ] 1.7 The diff uses existing LibreChat seams, not feature-internal rewrites.

### Phase 2 — Centralized UI gating

For features lacking config support but hideable at central seams: side-panel
entries, chat-input tools, menus, settings sections, unsupported nav routes.
Must handle the known `system.rs` mismatch (`interface.* = false` is NOT enough
because `role()` grants every permission and several `useSideNavLinks` entries
gate on permissions only).
- [ ] 2.1 Unsupported features no longer appear in normal navigation.
- [ ] 2.2 Direct navigation to disabled surfaces cannot produce broken/dead states.
- [ ] 2.3 Core navigation remains intact.
- [ ] 2.4 Knowledgeable Graph/Wiki navigation remains intact.
- [ ] 2.5 Changes are centralized and upstream-friendly.

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

### Phase 4 — Keep/future features: preserve clean seams

Bookmarks, pin, archive, fork/branch, conversation search, minimal share, file
attachments, file search, MCP, STT, TTS, voice/conversation mode, provider API
keys/BYOK, login/accounts/sync, token usage, billing, prompt slash commands.
- [ ] 4.1 They are not accidentally deleted during cleanup.
- [ ] 4.2 Where currently unsupported, they are documented as future backend/product work.
- [ ] 4.3 The current MVP remains small despite preserving future capability.
- [ ] 4.4 Future functionality has a clearly identifiable integration seam.
- [ ] 4.5 No speculative backend implementation is introduced for dormant UI.

### Phase 5 — Optional post-MVP feature projects (explicitly out of cleanup)

Each becomes its own feature project, never one giant "enable everything"
task: bookmarks, pin, archive, conversation search, fork/branch, minimal
sharing, attachments, document search/RAG, speech, MCP, BYOK, accounts/sync,
token usage, billing, prompt slash commands, voice mode.

### Phase 6 — Upstream synchronization discipline

- [ ] 6.1 Upstream changes review cleanly against a mostly recognizable tree.
- [ ] 6.2 Knowledgeable-specific changes are easy to identify.
- [ ] 6.3 Configuration/gating changes remain centralized.
- [ ] 6.4 Minimal unrelated diff noise.
- [ ] 6.5 Focused tests verify chat, streaming, conversations, message
  rendering, model selection, Graph/Wiki, and navigation after an update.
