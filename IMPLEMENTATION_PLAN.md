# IMPLEMENTATION_PLAN.md

## Implementation Graph

```
                    ┌───────────────┐
                    │ Project shell │
                    └───────┬───────┘
                            │
                     ┌──────┴──────┐
                     ↓             ↓
              Data model       Chat Controller
                     │             │
                     ↓             ↓
               Repository      Tooling (LLM)
                     │             │
                     └──────┬──────┘
                            ↓
                       E2E Validation
```

## Current Task Status

| ID | Description | Dependencies | Status |
| :--- | :--- | :--- | :--- |
| T1 | Reactive Tools implementation | - | COMPLETE |
| T2 | SSE Stream robustness | - | COMPLETE |
| T3 | E2E Playwright setup | - | COMPLETE |
| T4 | CORS hardening | - | COMPLETE |
| T5 | Manual E2E Validation | T3 | COMPLETE |
| T7 | Fix Tool Calling Engine | T1 | COMPLETE |
| T8 | Graph neighborhood API (`GET /v1/graph/neighborhood`, bounded CTE + learner confidence + 503) | - | COMPLETE |
| T9 | Graph Visualizer frontend — neighborhood view (`GraphExplorer` + `GraphPanel` + `graphClient` via `/api/graph/neighborhood`; semantic vs dependency distinction, confidence visibility, review/weak view; mounted in side nav; chat remains primary) | T8 | COMPLETE (28 Jest green; live browser E2E 10/10 vs seeded backend) |
| T10 | M7 backend annotations (`domain::annotation` matcher + `annotate_turn` + `concept_annotations` SSE frame) | - | COMPLETE (domain 7 + app 3 + router 1 tests green; workspace green) |
| T11 | M7 frontend (annotation store + SSE consume branches + `remarkConceptHighlight` + `<ConceptHighlight>` + pipeline specs) | T10 | COMPLETE (44 Jest green; live browser chat E2E 8/8) |
| T12 | v2 generation-protocol adapter (start ticket + `stream/:id` + `status/:conversationId` + roles + endpointType fallback) | - | COMPLETE (router 20 green; browser chat boot-to-badge E2E green) |
| T13 | M4 tutor orchestration (ADR-003 policy + `get_dependencies` + `tool_progress` frames + `StubLlmClient` loop tests) | - | COMPLETE (workspace green) |
| T14 | `tool_progress` UI consumer (`ToolActivity` mounted in chat messages) | T13 | COMPLETE (108 Jest green across knowledgeable + neighbors; tsc clean) |
| T15 | M5 proposals + admission (schema repair, gates, atomic admit + audit, `ConceptRef` to domain) | - | COMPLETE (9 candidate + 3 tool-loop tests; workspace green) |
| T16 | M6 observations + confidence (validation, apply txn + review sync, decay pass) | - | COMPLETE (6 observation + 2 decay tests; workspace green) |
| T17 | Keyed-LLM validation (Gemini tool willingness, repair pivot, arg robustness) | - | COMPLETE (real-model turns green; 1 parser bug + key hygiene fixed) |
| T18 | M3 providers (truthful models, per-turn dispatch, Ollama base URL, BYOK apiKey) | - | COMPLETE (plan unit + router tests; workspace green; live-Ollama gate open). NOTE 2026-09-23: Ollama provider since removed (2026-09-22, mobile-first) — generic `with_base_url` client retained, `ollama*` names fall through to boot default (locked by `ollama_names_fall_through_to_boot_default`). |
| T19 | Keyed browser session (real Gemini turn → tools → weak badge + activity) | - | COMPLETE (6/6 CDP checks). RESOLVED 2026-09-23: live default is `gemini-3.1-flash-lite` (via `GEMINI_MODEL` env; code fallbacks still name `gemini-2.5-flash-lite` — cosmetic only while env is set). Re-proven live: real keyed turn in headless Chrome → 2 tool-activity items + 3 concept badges, 4/4 CDP checks (`/tmp/opencode/cdp-t19.js` uncommitted). Note: Google intermittently 503s (`UNAVAILABLE` demand spikes) — a first attempt failed server-side and the retry passed; harness-proof convos deleted after. |
| T20 | M8 Personal Wiki (migration + WikiService + endpoint + drawer + click wiring) | - | COMPLETE (domain 3 + service 5 + router 1 + 59 Jest; 8/8 browser E2E) |
| T21 | M10 packaging (scripts/dev, static dist serving, prod build, concurrency + shutdown) | - | COMPLETE (27 api tests; prod build green; 25-way concurrency clean; SIGTERM clean) |
| T22 | Graph explorer search (`GET /api/concepts/search` + panel search UI) | - | COMPLETE (router 1 + 64 Jest green) |
| T25 | Highlight click path + known-only chat highlights | - | COMPLETE (65 Jest; live keyed check: only known badges) |

## Next Steps
- T23 (COMPLETE): graph tab rehaul — SVG canvas (`mapLayout` + `MapCanvas`, successor names for the earlier `graphLayout`/`GraphCanvas`) integrated into the explorer (root-anchored, review-aware), lists kept as fallback. Code + 74 Jest green, tsc clean. Live check passed 2026-09-19 (CDP: `graph-canvas` SVG 440x440 with 3 canvas nodes + 2 edges for the Prime Number neighborhood).
- T24 (COMPLETE 2026-09-23): LibreChat product-surface policy + staged cleanup — canonical policy in `docs/agent-context/integration/librechat.md` §9–§13 and phased contracts in `docs/agent-context/roadmap_and_state.md` §4. Phase 1 (config-only: parameters/presets/prompts/bookmarks/multiConvo/temporaryChat/runCode/webSearch/fileSearch off; only modelSelect + sidePanel on) and Phase 2 (15-step permission revocation down to BOOKMARKS + SHARED_LINKS) both shipped with locking router tests; backend gaps filled by F-projects (pin/archive/duplicate/search/minimal-share). Direction unchanged: trim the visible product while preserving LibreChat's architecture (KEEP bookmarks/pin/archive/fork/search/minimal-share; FUTURE files/MCP/speech/BYOK/accounts/billing; REMOVE generic memories/enterprise surfaces from view).
- Fixed post-T21: unknown `/api/*` + `/v1/*` paths stay JSON 404 under a mounted web build (the SPA fallback served 200 HTML and crashed boot parsing). Covered by regression test.
- User-path E2E (canonical stack: `./start.sh` + seeded DB + keyed backend): keyed turn with 3 tool-activity items, weak badge (Factor · 30%), graph neighborhood renders — 6/6 green.
- Beta prep: commit hygiene DONE 2026-09-23 (tree clean; annotation fix plan landed as PRs #1–3, CI rot fixed as PR #4, all merged). Keyed structured-generation proof DONE 2026-09-23 (live Gemini generated + cached the Fundamental Theorem page with riding annotations; seed reverted, proof convos deleted). Ollama dropped 2026-09-23 (not implementing; generic `with_base_url` retained for any future local relay).
- Fresh-clone rehearsal DONE 2026-09-22 (local-path clone = exact tree, env straight from `.env.example`, keyless): cold backend build 1m12s exit 0; boot smoke green (health/config/local-tutor-only models/empty convos); keyless turn streams 36 frames with truncation-fallback title intact; parallel `npm install` exit 0; prod `npm run build` 10.85s exit 0; single-origin serve proven (`/` 200 HTML, `/api/config` JSON precedence, unknown `/api/*` JSON 404). Finding fixed in the same pass: `.env.example` referenced retired `gemini-2.5-flash-lite` (T19) — now the live default. Scratch destroyed.
