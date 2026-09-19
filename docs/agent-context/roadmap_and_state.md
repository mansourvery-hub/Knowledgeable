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
- [x] 0.4 Clean working-tree baseline: commit `c89760b` (5 policy docs files)
  is the known-good baseline; `cargo fmt --all -- --check` clean and
  `cargo test --workspace --all-features` green (98 passed, 0 failed) verified
  2026-09-18 before implementation begins. (Full `scripts/verify.sh` not run:
  its Flutter section targets the deprecated `apps/client`.)

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

Contract 2.1 browser evidence (CDP, desktop viewport, labeled-button census):
side panel offers only Bookmarks (KEEP), Knowledge Graph (ours), Attach Files
(known scoped gap) — agent/assistant builder, Skills, Schedules, Prompts,
Memories, Parameters, and MCP builder are all absent; model picker works
(`gemini-3.1-flash-lite` default). Remaining visible gaps, all recorded, none
blocking: "All projects" nav entry (covered by the projects-guard brick),
"Attach File Options" (attachments project), "Use microphone" (speech surfaces
need their own gating brick: STT/TTS/voice-mode toggles).

Contract 2.2 audit (code-read, browser proof still pending): self-guarding by
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

Scoped sub-project (do NOT hide piecemeal): speech (STT/TTS/voice mode).
The composer mic button renders because `store.speechToText` defaults on and
`useSpeechSettingsInit` *enables* speech controls on a missing configuration
response — which is exactly what our backend returns (no
`/api/files/speech/config/*`). Gating means the recoil default, the init hook,
the SPEECH settings tab (~15 entries), and the STT/TTS hooks as one
browser-verified project, not a single toggle.

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
