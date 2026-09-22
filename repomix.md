# repomix.md — Adapted Repomix Commands for Knowledgeable

> Never run vanilla `repomix` in this repo. It packs the entire vendored
> LibreChat tree (`apps/web` is tracked: 2,923 files, 2,027 under
> `client/src/` alone), the deprecated Flutter client (`apps/client/`),
> and — worst of all — the untracked, non-gitignored `models.json` and
> `红楼梦.txt` (2.5 MB novel corpus). Measured 2026-09-20 with repomix 1.18.0.
>
> All commands below write to `/tmp/`, never into the repo (clean worktree rule,
> see `HANDOFF.md` §8). Security check stays ON (default): secrets are never
> packed, and `.env` / `*.db*` are additionally listed in `--ignore` as
> defense-in-depth.

Common flags used by every command:

| Flag | Why |
| :--- | :--- |
| `--style markdown` | Human-readable; matches the `HANDOFF.md` §8 precedent. (Use `--style xml` if the receiving model prefers Claude-style XML tags.) |
| `-o /tmp/<name>.md` | Keeps the repo tree clean. |
| `--header-text "..."` | Baked into the pack so the receiving LLM knows what was deliberately excluded and where to find it (deprecated Flutter client, the oversized SSE resume hook with its exact seam locations). |
| `--ignore "..."` | Shared exclusion list, see §5. Belt-and-braces on top of `.gitignore` (which repomix already respects). |

Line numbers (`--output-show-line-numbers`) are deliberately OFF: measured
~+120k tokens for no irreplaceable value (file paths + directory tree are
enough for citation). `--compress` is deliberately OFF: it strips function
bodies the coding LLM needs.

---

## 1. Full pack — whole system (recommended default)

**Size (measured): 168 files, ~231k tokens. Needs a 256k+ context model**
(Gemini, GPT-4.1, Kimi K2, Claude with extended context). For smaller
contexts use §2 / §3.

Scope: product docs + canonical agent-context + full Rust backend
(6 crates incl. tests) + migrations + run scripts + your entire
`src/knowledgeable/` frontend + only the 15 upstream LibreChat seam files
this codebase actually touches + minimal web configs.

```bash
repomix --style markdown -o /tmp/knowledgeable-pack.md \
--header-text "Knowledgeable: Rust/Axum+SQLite tutor backend in crates/ (api, application, domain, tutor, llm, infrastructure) + vendored LibreChat frontend in apps/web. Frontend included ONLY as apps/web/client/src/knowledgeable/** plus the listed upstream seam files; ~2000 other upstream files excluded. Deprecated Flutter client apps/client/ excluded - do not build on it. SSE resume hook useResumableSSE.ts excluded by size (190KB vendored); Knowledgeable touchpoints there are the store imports and the concept_annotations/tool_progress dispatch branches (grep concept_annotations in that file). Secrets (.env), SQLite (*.db), logs, conversations/, models.json, corpus txt excluded." \
--include "AGENTS.md,PRODUCT.md,MVP.md,ARCHITECTURE.md,IMPLEMENTATION_PLAN.md,QUALITY.md,TEST_STRATEGY.md,README.md,HANDOFF.md,CHANGELOG.md,TASK_TRACTION_AND_DISCOVERY.md,knowledgeable_blueprint.md,system_policy.txt,Cargo.toml,rust-toolchain.toml,rustfmt.toml,.env.example,start.sh,scripts/dev,scripts/verify.sh,.github/workflows/ci.yml,docs/**,crates/**,migrations/**,apps/web/package.json,apps/web/client/package.json,apps/web/client/vite.config.ts,apps/web/client/src/knowledgeable/**,apps/web/client/src/routes/ChatRoute.tsx,apps/web/client/src/hooks/Nav/useSideNavLinks.ts,apps/web/client/src/hooks/Files/useUploadOptions.ts,apps/web/client/src/hooks/SSE/useSSE.ts,apps/web/client/src/hooks/SSE/useAdaptiveSSE.ts,apps/web/client/src/hooks/SSE/index.ts,apps/web/client/src/components/Nav/Settings/registry.tsx,apps/web/client/src/components/Nav/Settings/types.ts,apps/web/client/src/components/Chat/Input/ChatForm.tsx,apps/web/client/src/components/Chat/Messages/HoverButtons.tsx,apps/web/client/src/components/Chat/Messages/Content/Markdown.tsx,apps/web/client/src/components/Chat/Messages/Content/markdownConfig.ts,apps/web/client/src/components/Projects/ProjectsView.tsx,apps/web/client/src/components/Projects/ProjectWorkspace.tsx" \
--ignore ".env,**/*.db*,**/*.sqlite*,**/*.log,conversations/**,models.json,红楼梦.txt,Cargo.lock,**/package-lock.json,apps/client/**,apps/web/**/dist/**,apps/web/**/coverage/**,**/node_modules/**,**/junit.xml,apps/web/client/vite-output.log,**/*.tsbuildinfo,target/**,apps/web/client/src/hooks/SSE/useResumableSSE.ts"
```

Include groups, in order:

1. **Product/state docs** — `AGENTS.md,PRODUCT.md,MVP.md,ARCHITECTURE.md,
   IMPLEMENTATION_PLAN.md,QUALITY.md,TEST_STRATEGY.md,README.md,HANDOFF.md,
   CHANGELOG.md,TASK_TRACTION_AND_DISCOVERY.md,knowledgeable_blueprint.md,
   system_policy.txt`. Product intent, MVP scope, invariants, roadmap/state,
   tutor prompt. The LLM's "why".
2. **Build/run configs** — `Cargo.toml,rust-toolchain.toml,rustfmt.toml,
   .env.example,start.sh,scripts/dev,scripts/verify.sh,.github/workflows/ci.yml`.
   How the system builds, runs, and is verified. (`.env.example` only —
   real `.env` holds a provider key and is ignored.)
3. **Canonical specs** — `docs/**` (agent-context architecture, data models,
   LibreChat integration matrix §9–§13, roadmap, tech rules, plus the two
   top-level handoff logs).
4. **Backend** — `crates/**` (api, application, domain, tutor, llm,
   infrastructure: source, bins, inline tests) + `migrations/**` (SQLite
   schema). The LLM's "how". Tests are kept: they pin the API/behavior
   contracts (e.g. `crates/api/src/routes/librechat/tests.rs`, ~11.6k tokens,
   documents adapter behavior).
5. **Your frontend** — `apps/web/client/src/knowledgeable/**` (all 35 files:
   components, stores, API clients, plugins, specs).
6. **Upstream seams** — the 15 vendored files Knowledgeable actually modifies
   (each verified via `rg knowledgeable` / gate-flag grep), see table in §4.
7. **Web configs** — `apps/web/package.json,apps/web/client/package.json,
   apps/web/client/vite.config.ts` (workspace layout, proxy, build).

---

## 2. Brain-only pack — backend + docs (~154k tokens)

Same flags as §1; swap the `--include` for the backend subset and keep the
shared `--ignore`. **Fits Claude 200k.** Use when the task is backend-only
(tutor orchestration, tools, graph services, API routes, migrations) and the
frontend is irrelevant.

```bash
repomix --style markdown -o /tmp/knowledgeable-brain.md \
--header-text "Knowledgeable backend-only pack: Rust/Axum+SQLite (crates/ + migrations/) plus product docs and canonical agent-context specs. Frontend (apps/web) and deprecated Flutter client (apps/client/) excluded - see the full-pack command in repomix.md when UI context is needed. Secrets (.env), SQLite (*.db), logs, conversations/, models.json, corpus txt excluded." \
--include "AGENTS.md,PRODUCT.md,MVP.md,ARCHITECTURE.md,IMPLEMENTATION_PLAN.md,QUALITY.md,TEST_STRATEGY.md,README.md,HANDOFF.md,CHANGELOG.md,TASK_TRACTION_AND_DISCOVERY.md,knowledgeable_blueprint.md,system_policy.txt,Cargo.toml,rust-toolchain.toml,rustfmt.toml,.env.example,start.sh,scripts/dev,scripts/verify.sh,.github/workflows/ci.yml,docs/**,crates/**,migrations/**" \
--ignore ".env,**/*.db*,**/*.sqlite*,**/*.log,conversations/**,models.json,红楼梦.txt,Cargo.lock,**/package-lock.json,apps/client/**,apps/web/**/dist/**,apps/web/**/coverage/**,**/node_modules/**,**/junit.xml,apps/web/client/vite-output.log,**/*.tsbuildinfo,target/**"
```

---

## 3. Surface-only pack — frontend + seams (~108k tokens)

Same flags as §1; narrow `--include` to product docs + agent-context + the
frontend subset. **Fits 128k models** (GPT-4o-class, DeepSeek-class). Use for
UI tasks (GraphExplorer, WikiDrawer, badges, settings gating, nav) where the
Rust backend is background.

```bash
repomix --style markdown -o /tmp/knowledgeable-surface.md \
--header-text "Knowledgeable frontend-only pack: product docs plus the Knowledgeable-owned UI (apps/web/client/src/knowledgeable/**) and the upstream LibreChat seam files it touches. Rust backend (crates/) excluded except via the documented API contracts - see the full-pack or brain-only commands in repomix.md when backend context is needed. ~2000 other vendored upstream files excluded; deprecated Flutter client excluded; secrets/DBs/logs excluded." \
--include "PRODUCT.md,MVP.md,ARCHITECTURE.md,docs/agent-context/**,apps/web/package.json,apps/web/client/package.json,apps/web/client/vite.config.ts,apps/web/client/src/knowledgeable/**,apps/web/client/src/routes/ChatRoute.tsx,apps/web/client/src/hooks/Nav/useSideNavLinks.ts,apps/web/client/src/hooks/Files/useUploadOptions.ts,apps/web/client/src/hooks/SSE/useSSE.ts,apps/web/client/src/hooks/SSE/useAdaptiveSSE.ts,apps/web/client/src/hooks/SSE/index.ts,apps/web/client/src/components/Nav/Settings/registry.tsx,apps/web/client/src/components/Nav/Settings/types.ts,apps/web/client/src/components/Chat/Input/ChatForm.tsx,apps/web/client/src/components/Chat/Messages/HoverButtons.tsx,apps/web/client/src/components/Chat/Messages/Content/Markdown.tsx,apps/web/client/src/components/Chat/Messages/Content/markdownConfig.ts,apps/web/client/src/components/Projects/ProjectsView.tsx,apps/web/client/src/components/Projects/ProjectWorkspace.tsx" \
--ignore ".env,**/*.db*,**/*.sqlite*,**/*.log,conversations/**,models.json,红楼梦.txt,Cargo.lock,**/package-lock.json,apps/client/**,apps/web/**/dist/**,apps/web/**/coverage/**,**/node_modules/**,**/junit.xml,apps/web/client/vite-output.log,**/*.tsbuildinfo,target/**,apps/web/client/src/hooks/SSE/useResumableSSE.ts"
```

---

## 4. Upstream seam files — why each one is included

These are the only vendored files packed. Each has a verified Knowledgeable
touchpoint (import, gate flag, or component mapping). Everything else under
`apps/web/` (~2,000 files) is excluded.

| File | Touchpoint |
| :--- | :--- |
| `routes/ChatRoute.tsx` | Hosts the `WikiDrawer` (single mount point). |
| `hooks/Nav/useSideNavLinks.ts` | Graph + Personal Wiki nav entries; attachments gate. |
| `hooks/Files/useUploadOptions.ts` | Forces `uploadsDisabled` (central attachments gate). |
| `hooks/SSE/useSSE.ts` | Consumes `concept_annotations` + `tool_progress` frames. |
| `hooks/SSE/useAdaptiveSSE.ts` + `index.ts` | Dispatcher/barrel for the SSE layer (tiny). |
| `components/Nav/Settings/registry.tsx` + `types.ts` | `manageFiles`/`show` gates, SPEECH tab removal, entry-level `show` flags. |
| `components/Chat/Input/ChatForm.tsx` | Composer gates (attach button, mic, autoplay). Largest seam (~9k tokens) but the core input surface. |
| `components/Chat/Messages/HoverButtons.tsx` | Per-message speak-button gate. |
| `components/Chat/Messages/Content/Markdown.tsx` + `markdownConfig.ts` | Registers `<ConceptHighlight>` + remark plugin wiring. |
| `components/Projects/ProjectsView.tsx` + `ProjectWorkspace.tsx` | Unconditional redirect-to-chat guards. |

Deliberately excluded despite a touchpoint:
`hooks/SSE/useResumableSSE.ts` — 190 KB / ~38k tokens vendored, only 2
import lines + one dispatch branch are ours (`concept_annotations` /
`tool_progress` at ~line 2172). Re-add it explicitly if the task is SSE
resume/reconnect internals.

---

## 5. Shared `--ignore` list — what is excluded and why

| Pattern | Reason |
| :--- | :--- |
| `.env` | Real provider key. Never pack secrets (security check also guards this). |
| `**/*.db*,**/*.sqlite*` | Local SQLite files (root + `apps/*`), incl. `-wal`/`-shm`. Binary + user test data. |
| `**/*.log` | `backend.log`, `server.log`, `vite-e2e.log` — run noise. |
| `conversations/**` | Local transcripts (gitignored, but explicit). |
| `models.json` | Untracked 30 KB model-dump; **not gitignored** — vanilla repomix would pack it. |
| `红楼梦.txt` | Untracked 2.5 MB novel corpus; **not gitignored** — the single most expensive vanilla-repomix trap. Never stage or touch (see `HANDOFF.md`). |
| `Cargo.lock,**/package-lock.json` | Lockfiles: huge, zero design signal. |
| `apps/client/**` | Deprecated Flutter client (roadmap §2: do not build on it). Includes `build/`, `android/`, `ios/`, `node_modules/`. |
| `apps/web/**/dist/**,apps/web/**/coverage/**` | Build output + generated HTML coverage reports. |
| `**/node_modules/**` | Dependencies (also gitignored/default-ignored; listed explicitly). |
| `**/junit.xml,apps/web/client/vite-output.log,**/*.tsbuildinfo` | Test/build byproducts (note: Jest rewrites `junit.xml` on every run). |
| `target/**` | 13 GB Rust build dir (also default-ignored; listed explicitly). |
| `apps/web/client/src/hooks/SSE/useResumableSSE.ts` | See §4. Only in the full (§1) and surface (§3) commands. |

---

## 6. Maintenance

- **Re-run after each brick** so the receiving LLM's picture stays current;
  outputs go to `/tmp/`, never commit them.
- **New upstream touch?** If you add a `knowledgeable` import outside
  `src/knowledgeable/`, append that file to the `--include` lists in §1 and
  §3 and add a row to §4. Detection:
  `rg -l knowledgeable apps/web/client/src --glob '!apps/web/client/src/knowledgeable/**'`
- **Size check before feeding a model:** append `--token-count-tree` (per-file
  cost preview) or `--token-budget <N>` (non-zero exit if output exceeds N
  tokens — tripwire against accidentally widened patterns).
- **Per-task packs:** for narrow tasks, prefer a tiny scoped pack in the
  `HANDOFF.md` §8 style (e.g. wiki-only, ~50 files) over these whole-system
  commands.
