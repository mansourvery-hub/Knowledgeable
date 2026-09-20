# Handoff — Knowledgeable LibreChat cleanup + beta (new session)

> Paste this whole file into a fresh session with access to this repository.
> It contains everything the previous session learned, including harness
> gotchas that cost real debugging time.

## 1. What this is

Knowledgeable = focused AI tutoring app: vendored LibreChat React frontend
(`apps/web`) + Rust/Axum backend + SQLite (`crates/`), single binary serves
the built client. The tutor tracks learner confidence per concept in a
knowledge graph and teaches at the learner's frontier. Product direction:
**trim the visible LibreChat surface while preserving upstream architecture**
— never a rewrite.

## 2. Read first, in this order

1. `PRODUCT.md`, `MVP.md`, `ARCHITECTURE.md`, `IMPLEMENTATION_PLAN.md`
2. `docs/agent-context/integration/librechat.md` §9–§13 (canonical feature
   matrix, backend boundary, 13 upstream constraints — this overrides older notes)
3. `docs/agent-context/roadmap_and_state.md` §4 (staged cleanup contracts) and
   §5 (beta feedback log F1–F8 — live user reports, partially triaged)
4. `docs/agent-context/architecture.md`, `TEST_STRATEGY.md`, `QUALITY.md`

## 3. Current state (all pushed to `origin/main`)

- **Grants**: `/api/roles/USER` returns exactly `BOOKMARKS` + `SHARED_LINKS`
  (both deliberate KEEP holds). 15 revocations, one per commit, each with a
  blast-radius note in `system.rs` comments and the roles test.
- **Deletions** (Phase 3 isolated rows only): Insights view, Memories panel,
  Schedules panel. Ledger in roadmap §4 records what is entangled and must
  NOT be deleted wholesale (Skills/Prompts/Agents/132-file SidePanel/Agents).
- **Gates closed with browser evidence**: M2 (3/3), M9 (3/3), T23 canvas,
  M1 (2/3 — box 1 open: handled boot-probe noise, settled as boot-only).
- **Features added**: node→wiki button in Graph Explorer; projects routes
  redirect to chat; `defaultTemporaryChat` gated in settings registry.
- **Backend fix**: credential validation precedes conversation creation (no
  empty shells on 400).
- **Beta P0 done**: prod build green, single-origin serving proven.
- **Specced, not built**: Phase 5a Personal Wiki browser (roadmap §4).
- Worktree is clean except two pre-existing untracked files (`models.json`,
  `红楼梦.txt`) — **never stage or touch them**.

## 4. How to work (standing rules from the user)

- **One brick at a time**: smallest meaningful change → verify → commit →
  report → stop. Auto-commit finished bricks without asking. **Push only on
  explicit request.** Never `rm -rf` a feature directory first.
- Verify by execution (tests, curl, CDP), never by reasoning alone. Do not
  claim builds/tests passed unless run.
- `cargo fmt` per touched crate; full `tsc --noEmit` must stay at exactly the
  **25 pre-existing upstream errors** (zero in touched files).
- Jest rewrites `apps/web/client/junit.xml` as a side effect — always restore
  it, never commit it. Frontend suites are slow (~160s for 2 small files).
- Check `git status`/`git diff` before every commit; stage only intended files.

## 5. Live environment (assume dead unless re-verified)

- Backend `:3000` + Vite `:3090` + headless Chrome CDP `:9222` were running
  via `nohup` from prior sessions. **They die when terminals exit**
  (`./start.sh` traps EXIT and kills children). Check all three with curl
  before trusting them; restart with the commands in §6 of the last session's
  notes if needed. The user also runs `./start.sh` themselves (it `fuser -k`s
  the ports, killing your background servers).
- **The SQLite DB is shared with the user's manual testing.** Never delete
  conversations you didn't create. Test litter is yours to clean via
  `DELETE /api/convos`.
- `.env` holds a real provider key. **Never print env values.** Keyless-model
  probes (`gpt-4o-mini`) must 400 explicitly.
- CDP harness lives in `/tmp/cdp-*.js` (NOT committed): raw WebSocket CDP
  via `require('/home/mohamed/Desktop/Github/Knowledgeable/apps/web/node_modules/ws')`.
  Lessons already paid for: match responses by captured message id (closure
  over mutable id hangs); tolerate both `result.value` and
  `result.result.value` shapes; headless defaults to a mobile viewport that
  hides the sidebar (always set 1440x900 emulation); click `button` testids,
  not `<li>` wrappers with identical text; regex markers can false-positive
  inside error strings; static dump-DOM lags live paint (endpoint label);
  virtual-time-budget distorts polling intervals (use wall-clock sleeps).
- Ask the user before: pushing, multi-minute builds, anything touching
  provider keys/secrets, or product calls (listed in §6).

## 6. Open queue (highest value first)

1. **Phase 5a wiki browser** — specced, user explicitly deferred then may
   request. Backend list contract first, then view, then browser pass.
2. **F6 bold-vs-badge** — tutor writes `**concept**` markdown that impersonates
   badges. Candidate: prompt tweak against bolding bare concept names.
3. **F8 tooltip call** — hover popups with percentages: keep / de-grease /
   debug-toggle. Needs the user's decision.
4. **Scoped projects** — attachments gating + speech gating (root causes
   mapped in roadmap; each needs a browser-verified pass, do not piecemeal).
5. **F5 `log_observation` arg-parse failure** — harden tool-arg parsing.
6. **M1 box 1** — accept documented probe noise or schedule suppression.
7. **Fresh-clone rehearsal** — scratch clone → `.env.example` → cold build →
   boot smoke (never done; ~15 min machine time).
8. **Phase 3 entangled rows** — do not touch without a new disentangling plan.

## 7. Standing product facts (don't re-derive)

- `role()` absent grant == denied client-side (`useHasAccess` strict `=== true`).
- `interface.* = false` alone does NOT hide side-panel entries (they gate on
  permissions) — the mismatch that motivated Phase 2.
- Concept badges render for KNOWN (≥0.80) only, live turns only: annotations
  are in-memory, **reload wipes all badges** (filed as project F7).
- Wiki pages generate on miss at mastery ≥0.70; drawer handles
  loading/not-ready/error states.
- `routes/index.tsx` is outside the approved touchpoint list — prefer component
  self-guards (`<Navigate>`) as done for Skills/Prompts/Marketplace/Projects.
- `apps/web/client/src/knowledgeable/` is our protected boundary; the Rust
  adapter stays narrow (no route without a product requirement).

## 8. Agent context packing (repomix)

Never run vanilla `repomix` — it packs the entire vendored LibreChat tree
(tens of thousands of files) and thrashes this laptop. Scope with `--include`
to exactly what the receiving agent needs. Example: wiki-research pack
(52 files, ~72k tokens):

```
repomix --style markdown -o /tmp/wiki-research-pack.md --include "PRODUCT.md,MVP.md,ARCHITECTURE.md,IMPLEMENTATION_PLAN.md,QUALITY.md,TEST_STRATEGY.md,system_policy.txt,docs/agent-context/**,apps/web/client/src/knowledgeable/**,apps/web/client/src/hooks/Nav/useSideNavLinks.ts,crates/application/src/wiki_service.rs,crates/api/src/routes/librechat/wiki.rs,crates/api/src/routes/librechat/concepts.rs,migrations/**"
```

- Keep output in `/tmp/`, never in the repo (clean tree).
- `--token-budget <N>` fails instead of melting when output exceeds N tokens.
- `--token-count-tree` previews per-file cost before packing.
- Security check runs by default; `.env`/`.db` are gitignored and excluded.
