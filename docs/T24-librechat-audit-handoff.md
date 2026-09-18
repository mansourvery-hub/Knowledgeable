# Handoff Prompt — T24: LibreChat Feature Audit (Keep vs Remove)

> Paste everything below the line into a capable model with access to this
> repository. No code changes in this task — analysis and recommendation only.

---

# Task: Audit every bundled LibreChat feature against the Knowledgeable product vision

## 1. Product vision (what we are)

**Knowledgeable** is an AI tutor that teaches from the learner's frontier. Core loop:

```
ASK → UNDERSTAND THE LEARNER → TEACH FROM THE FRONTIER → OBSERVE → UPDATE GRAPH → CRYSTALLIZE
```

Authoritative context — read these first, in this order:

1. `PRODUCT.md` — problem, users, journeys, non-goals (no curricula generation, no social features)
2. `MVP.md` — scope and acceptance criteria
3. `ARCHITECTURE.md` — layer invariants (Rust/SQLite owns state; web is presentation)
4. `docs/agent-context/integration/librechat.md` — why LibreChat is vendored frontend-only, the narrow integration seams (§7), and what must never come back (Node/Express backend, MongoDB, Redis, MeiliSearch)
5. `docs/agent-context/roadmap_and_state.md` — milestones M0–M10 (all backend-complete) and what remains

Key architectural facts:

- The **Rust Axum backend + SQLite is the entire system of record**. The web client is presentation only.
- The backend implements a **deliberate subset** of LibreChat's `/api/*` surface (see `crates/api/src/routes/librechat/`). Every client feature whose endpoint is unimplemented degrades in confusing ways (spinners, 404 noise, dead buttons).
- Knowledgeable-only UI lives in `apps/web/client/src/knowledgeable/` and must stay there. Upstream files may change **only** at the seams listed in `integration/librechat.md` §7.
- Single-user local-first. No teams, no orgs, no billing, no marketplace, no auth providers.

## 2. What to inventory

Enumerate **every user-facing feature currently bundled** in the vendored client. Concrete places to look (non-exhaustive — you are expected to find more):

- `apps/web/client/src/routes/` — routable views (`/agents`, `/skills`, `/prompts`, `/projects`, `/insights`, `/search`, `/share`, …). For each: what is it, who is it for upstream, does our backend serve its endpoints?
- `apps/web/client/src/hooks/Nav/useSideNavLinks.ts` + `UnifiedSidebar/` — every sidebar/panel entry and what backs it.
- `apps/web/client/src/components/SidePanel/` — every panel (Agents, MCPBuilder/Tools, Bookmarks, Memories, Files, Parameters, Schedules, Prompts, Skills, …).
- `apps/web/client/src/components/Chat/Menus/` + endpoint/model selectors — what options exist and which resolve against our `/api/endpoints` + `/api/models`.
- Settings modals, plugin store, MCP servers UI, speech/TTS, file upload/search, web search, code execution (`runCode`), multi-convo, temporary chat, presets,_export/import, shared links, forks/branches, message feedback, token usage/billing surfaces.
- `apps/web/packages/*` — shared libraries that exist only to serve trimmed features.

For backend cross-check: list which `/api/*` routes each feature calls (grep the data-provider + hooks), and mark each as **served / stubbed / 404** by `crates/api/src/routes/librechat/`.

## 3. Evaluation criteria

For each feature, judge against **all five** lenses and say which dominate the verdict:

1. **Vision fit** — does it serve frontier tutoring for a solo learner? (Chat, streaming, markdown/LaTeX/code rendering, conversation history, model picking, and our Knowledgeable extensions are the core.)
2. **Backend reality** — is its API surface implemented? Features that 404 or half-work are worse than absent (they erode trust).
3. **Distraction cost** — does it clutter nav, settings, or menus away from chat-first learning?
4. **Maintenance risk** — does keeping it widen the upstream-merge surface or force backend endpoints we will never build well?
5. **Reversibility** — can it come back later cheaply if the vision expands (prefer *disable via config* over deletion where the code is entangled)?

## 4. Deliverable

A single report with:

1. **Feature table** — one row per feature: `Feature | Where it lives (files/routes) | Backend status (served/stubbed/404) | Verdict: KEEP / DISABLE (config) / REMOVE | One-sentence rationale citing a criterion above`.
2. **Kill list** — the concrete removal plan ordered by value/effort: exact files/routes/config flags to touch, and for each, what the user sees change.
3. **Keep list** — what stays and why, plus any backend endpoint it still needs (flag gaps as follow-up tasks, e.g. "needs real `/api/files`" — do NOT design them here).
4. **Risks** — anything whose removal could break the core chat loop, and how to verify it didn't (which Jest suites + which manual click-path).
5. **Out of scope, explicitly** — do not touch `src/knowledgeable/`, do not touch the Rust backend, do not touch migrations, do not propose new features.

## 5. Rules of engagement

- **Read-only task.** Do not edit, create, or delete any repo files. Do not run builds, installs, or servers.
- Verify claims by reading code, not by guessing: every "backend status" cell must name the route handler file or the missing route.
- Be ruthless but honest: if you're unsure whether something serves the vision, mark it `REVIEW` with the question, don't force a verdict.
- Keep the report under ~150 lines excluding the table. The table may be long — that's fine.
