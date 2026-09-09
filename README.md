# Knowledgeable — AI Tutor from Your Frontier

> **Status:** Phase 0 — Project foundation complete (SQLite-native). See `docs/agent-context/roadmap_and_state.md`.

Knowledgeable is an AI tutoring application whose core loop is:

```
ASK → UNDERSTAND THE LEARNER → TEACH FROM THE FRONTIER → OBSERVE → UPDATE GRAPH → CRYSTALLIZE
```

The learner graph is the persistent model behind tutoring, not the UI. See `knowledgeable_blueprint.md`.

## Stack (robust, modern, long-lived)

- **Client:** Flutter 3.47 + Dart 3.13, Riverpod, go_router, Drift + SQLite, Dio, freezed/json_serializable
- **Backend:** Rust stable (Tokio, Axum 0.7, SQLx SQLite, tracing, thiserror)
- **DB:** SQLite (canonical, WAL + foreign_keys + busy_timeout, file-local) — PostgreSQL is **not** in default dev/deploy; can be introduced later without domain rewrite
- **Infra:** `fvm` for Flutter pinning, `sqlx-cli --features sqlite` for migrations, native binaries — **no Docker required**

All choices follow: **robust, fast, modern, and not prone to early obsolescence** (same rationale as Flutter).

## Monorepo Layout

```
/
├── apps/client            # Flutter (mobile + web)
│   ├── lib/app            # bootstrap, router, theme
│   ├── lib/core           # api_client, app_database (Drift cache)
│   └── lib/features       # chat, sessions, review, graph
├── crates/
│   ├── api                # Axum routes (thin)
│   ├── application        # orchestration + transactions
│   ├── domain             # pure invariants (concept, relation, confidence, decay, validation, ports)
│   ├── tutor              # tools / prompts
│   ├── llm                # provider-neutral traits
│   └── infrastructure     # SQLx SQLite, config, telemetry (WAL/FKs)
├── migrations/            # SQLite (sqlx migrate, STRICT tables, JSON validity, bounded depth via CTEs)
├── docs/agent-context/    # canonical spec (SQLite-native)
└── scripts/verify.sh
```

Domain stays pure (no Axum/SQLx/LLM). Application depends on domain ports, not infrastructure implementations. Fewer meaningful crates over thin abstraction.

## Quick Start — No Docker, No Postgres

### Prerequisites

- Rust stable (`pacman -S rust` or rustup), `cargo fmt` + `clippy`
- `fvm` + Flutter stable (or `flutter` on PATH) — pinned via `.fvmrc`
- `sqlx-cli` sqlite: `cargo install sqlx-cli --no-default-features --features sqlite`
- SQLite is file-local, auto-created — no daemon

### 1. Clone & env

```bash
git clone https://github.com/mansourvery-hub/Knowledgeable.git
cd Knowledgeable
cp .env.example .env   # DATABASE_URL=sqlite:knowledgeable.db is default
```

### 2. Database — zero setup

```bash
# SQLite file created automatically on first run or migrate
sqlx database create   # optional — create knowledgeable.db
sqlx migrate run       # applies migrations/20260909125009_initial_schema.sql (STRICT, TEXT UUIDs)

# Or just: cargo run -p api  → creates knowledgeable.db via SqliteConnectOptions::create_if_missing(true) + WAL/FKs
ls -lh knowledgeable.db*
```

No `docker compose`, no `initdb`, no `createdb`.

Health check:

```bash
cargo run -p api  # in one terminal — binds 0.0.0.0:3000, enables WAL, foreign_keys=ON, busy_timeout=5000
curl http://localhost:3000/health | jq
# {"status":"ok","version":"0.1.0","database_connected":true}
curl http://localhost:3000/health/db | jq
```

### 3. Backend

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo run -p api
```

### 4. Client

```bash
fvm flutter pub get
fvm flutter analyze
fvm flutter test
fvm flutter run           # or: fvm flutter run -d chrome
# SQLite cache in app is Drift, separate from backend file — sync is server-authoritative later
```

### 5. Full verification (CI baseline)

```bash
./scripts/verify.sh
# cargo fmt --check, cargo check/test/clippy, flutter analyze/test — no DB daemon needed
```

CI (`.github/workflows/ci.yml`) now runs without services: `sqlx-cli --features sqlite`, `DATABASE_URL=sqlite:knowledgeable.db`, `sqlx migrate run`.

## Architecture Invariants

- Flutter canonical client; Rust owns authoritative state; **SQLite** is source of truth (file-local, WAL); Drift SQLite is cache, not authority.
- `world_confidence >= 0.80` gates admission (is knowledge trustworthy?); `learner_confidence ∈ [0,1]` tracks understanding health (`≥0.95` healthy, else review-eligible) — independent.
- Semantic vs dependency edges distinct; `from depends_on to` canonical; bounded traversal (max 5, default 3) via recursive CTEs.
- Tutor: deterministic code owns state/validation/transactions; LLM reasons/navigates/proposes; sparse graph supported (tutor proposes missing prerequisites from frontier).
- See `docs/agent-context/architecture.md` + `data_models.md` for canonical names.

## Roadmap

Phase 0 (done, SQLite-native): workspace (6 crates), health, SQLite WAL/FK, Flutter bootstrap, CI without Postgres/Docker.  
Phase 1 next: Conversation → Tutor → LLM abstraction → SSE → Flutter chat UI (no vector search, no Docker).  
See `docs/agent-context/roadmap_and_state.md`.

## Contributing

- Run `cargo fmt` and `fvm dart format` before commit.
- Keep agent-context docs in sync with code; latest decisions override blueprint assumptions.
- Treat LLM output and user content as untrusted.

## License

MIT OR Apache-2.0
