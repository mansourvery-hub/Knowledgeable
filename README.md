# Knowledgeable — AI Tutor from Your Frontier

> **Status:** Phase 0 — Project foundation complete. See `docs/agent-context/roadmap_and_state.md`.

Knowledgeable is an AI tutoring application whose core loop is:

```
ASK → UNDERSTAND THE LEARNER → TEACH FROM THE FRONTIER → OBSERVE → UPDATE GRAPH → CRYSTALLIZE
```

The learner graph is the persistent model behind tutoring, not the UI. See `knowledgeable_blueprint.md`.

## Stack (robust, modern, long-lived)

- **Client:** Flutter 3.47 + Dart 3.13, Riverpod, go_router, Drift + SQLite, Dio, freezed/json_serializable
- **Backend:** Rust stable (Tokio, Axum 0.7, SQLx, PostgreSQL 16, tracing, thiserror)
- **DB:** PostgreSQL (authoritative) + `pgcrypto`, SQLite (disposable cache)
- **Infra:** `fvm` for Flutter version pinning, `docker-compose` for local Postgres, `sqlx-cli` for migrations

All choices follow the principle: **robust, fast, modern, and not prone to early obsolescence** (same rationale as Flutter).

## Monorepo Layout

```
/
├── apps/client            # Flutter (mobile + web)
│   ├── lib/app            # bootstrap, router, theme
│   ├── lib/core           # api_client, app_database
│   ├── lib/features       # auth, chat, sessions, review, graph
│   └── lib/data/state/widgets
├── crates/
│   ├── api                # Axum routes (thin)
│   ├── application        # orchestration + transactions
│   ├── domain             # pure invariants
│   ├── graph              # retrieval / traversal
│   ├── learner            # confidence / decay / review
│   ├── tutor              # tools / prompts
│   ├── llm                # provider-neutral traits
│   ├── validation         # world-confidence gate
│   └── infrastructure     # SQLx, config, telemetry
├── migrations/            # PostgreSQL (sqlx migrate)
├── docs/agent-context/    # canonical spec
├── scripts/verify.sh
└── docker-compose.yml
```

## Quick Start

### Prerequisites

- Rust stable (`rustup` or Arch `pacman -S rust`), `cargo fmt` + `clippy`
- `fvm` + Flutter stable (or `flutter` on PATH) — pinned via `.fvmrc`
- `sqlx-cli`: `cargo install sqlx-cli --no-default-features --features native-tls,postgres`
- PostgreSQL 16 — **either** `docker` + `docker-compose` **or** native `pacman -S postgresql`

### 1. Clone & env

```bash
git clone https://github.com/mansourvery-hub/Knowledgeable.git
cd Knowledgeable
cp .env.example .env   # edit DATABASE_URL if needed
```

### 2. Database

With Docker (recommended, reproducible, CI-identical):

```bash
docker compose up -d postgres
# or: docker-compose up -d
sqlx migrate run
```

Without Docker (Arch native):

```bash
sudo pacman -S postgresql
sudo -u postgres initdb -D /var/lib/postgres/data
sudo systemctl enable --now postgresql
createdb knowledgeable -U postgres
sqlx migrate run
```

Health check:

```bash
curl http://localhost:3000/health | jq
# {"status":"ok|degraded","version":"0.1.0","database_connected":true/false}
curl http://localhost:3000/health/db | jq
```

### 3. Backend

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo run -p api  # server on 0.0.0.0:3000 (degraded mode if DB down)
```

### 4. Client

```bash
# via fvm (pinned)
fvm flutter pub get
fvm flutter analyze
fvm flutter test
fvm flutter run           # or: fvm flutter run -d chrome

# or plain flutter if on PATH
flutter pub get && flutter analyze && flutter test
```

### 5. Full verification (CI baseline)

```bash
./scripts/verify.sh
# runs: cargo fmt --check, cargo check/test/clippy, flutter analyze/test
```

CI runs the same on GitHub Actions (`.github/workflows/ci.yml`) with a `postgres:16-alpine` service.

## Architecture Invariants

- Flutter is canonical client; Rust owns authoritative state; PostgreSQL is source of truth; SQLite is cache.
- `world_confidence` gates admission (`>=0.80`); `learner_confidence` tracks health (`>=0.95` healthy).
- Semantic vs dependency edges are distinct; `from depends_on to` is canonical.
- Tutor reasons via bounded tools; deterministic code validates/commits; LLM output is untrusted.
- See `docs/agent-context/architecture.md` + `data_models.md` for canonical names and contracts.

## Roadmap

Phase 0 (done): workspace, health, config, tracing, Flutter bootstrap, CI.  
Phase 1 next: conversations, SSE streaming, provider-neutral LlmClient.  
See `docs/agent-context/roadmap_and_state.md`.

## Contributing

- Run `cargo fmt` and `fvm dart format` before commit.
- Keep `architecture.md`, `data_models.md`, `tech_stack_and_rules.md` in sync with code.
- Treat LLM output and user content as untrusted data.

## License

MIT OR Apache-2.0
