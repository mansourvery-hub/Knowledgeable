# Knowledgeable — Tech Stack and Coding Rules

## 1. Required Stack

<stack>
Presentation Layer (`apps/web`):
- React 18 + TypeScript SPA vendored from LibreChat.
- Vite for fast compilation, HMR, and development proxying.
- Tailwind CSS for component styling.
- Recoil for global UI state and TanStack Query (@tanstack/react-query) for API data caching.
- Unified / Remark / Rehype / KaTeX pipeline for Markdown, LaTeX formulas, code highlighting, and AST plugins.
- Knowledgeable extensions isolated in `apps/web/client/src/knowledgeable/`.
- [DEPRECATED] Flutter client in `apps/client`.

Backend Core (`crates/`):
- Rust stable.
- Tokio async runtime.
- Axum HTTP/SSE server.
- Serde + serde_json for serialization.
- SQLx for SQLite access (`sqlite:knowledgeable.db`, `create_if_missing`, WAL mode, foreign keys enabled).
- SQLite as canonical authoritative database (file-local, WAL `journal_mode=WAL`, `foreign_keys=ON`, `busy_timeout=5000`, transactions for mutations).
- tracing + tracing-subscriber for structured observability.
- thiserror for typed internal/domain errors.
- anyhow only at application boundaries where contextual propagation is useful.

Tooling:
- Cargo workspace.
- rustfmt + clippy.
- npm / vite.
- CI runs Rust tests/checks and web build/typecheck.
</stack>

## 2. Explicitly Rejected Until Reconsidered

```text
- Running upstream LibreChat Node.js backend (mandates MongoDB, Redis, and MeiliSearch)
- Dedicated external graph database (Neo4j, Memgraph)
- PostgreSQL / pgvector / Redis / queues for default dev/deploy
- Client-side LLM provider SDKs (all LLM interactions must flow through backend tutor orchestrator)
- Brittle LLM HTML generation for concept highlighting (<span class="...">)
- Consumer subscription hijacking (ChatGPT Plus, Gemini Advanced session cookie extraction)
- Giant prebuilt universal ontology
- Complex multi-factor SRS algorithms in v1
```

The architecture may change only through an explicit architecture decision recorded in `docs/agent-context/architecture.md`.

---

## 3. Dependency Discipline

Allowed dependencies between layers:

```text
crates/api -> crates/application -> crates/domain
crates/tutor -> crates/domain
crates/llm -> crates/domain
crates/infrastructure -> crates/domain
```

Forbidden dependencies:

```text
crates/domain -> crates/api
crates/domain -> crates/application
crates/domain -> crates/infrastructure
crates/domain -> Axum
crates/domain -> SQLx
crates/domain -> LLM SDK
crates/tutor -> raw SQL
crates/api -> provider-specific LLM SDK
LLM provider adapter -> direct graph mutation
crates/application -> infrastructure concretions (must depend on domain ports)
```

---

## 4. Frontend Rules (`apps/web` - LibreChat Client)

### Structure & Isolation
- All Knowledgeable-specific features (Wiki drawer, Concept renderer, Graph visualizer) must live in `apps/web/client/src/knowledgeable/`.
- Upstream files must only be modified at designated integration seams:
  - `client/src/components/Chat/Messages/Content/markdownConfig.ts` (registering `remarkConcept` and `ConceptHighlight`).
  - `client/src/components/Nav/Nav.tsx` (adding Knowledgeable navigation items).
  - `client/src/routes/ChatRoute.tsx` (mounting slide-over drawers).
  - `client/vite.config.ts` (configuring backend proxy).

### Concept Highlighting
- Never attempt string-replacement on raw markdown or HTML.
- Always use the Unist Markdown AST visitor (`remarkConceptHighlight`).
- The visitor must inspect only narrative `text` nodes within paragraphs, list items, and blockquotes.
- The visitor must skip code blocks (`code`, `pre`), inline math (`inlineMath`), block math (`math`), and link URLs.

### Data Fetching & Caching
- Use `@tanstack/react-query` hooks defined in `@librechat/data-provider` or custom hooks in `apps/web/client/src/knowledgeable/api/`.
- Client storage is a cache only. Authoritative state resides exclusively in the backend SQLite database.

---

## 5. Rust Rules

### General
- `cargo fmt` is mandatory.
- `cargo clippy --all-targets --all-features -- -D warnings` is the baseline CI expectation.
- Avoid `unwrap()`/`expect()` in request/application paths.
- Handle `Option`/`Result` explicitly.
- Prefer enums over stringly typed state.
- Prefer small structs with explicit ownership.

### Error Handling
Use typed errors at module boundaries with `thiserror`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("validation failed")]
    Validation(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("not found")]
    NotFound(String),
    #[error("graph mutation rejected: {0}")]
    GraphMutationRejected(String),
    #[error("llm provider error: {0}")]
    LlmProvider(String),
    #[error("internal error: {0}")]
    Internal(String),
}
```

Client-facing error payloads expose stable HTTP status codes and JSON error objects, never raw database traces.

---

## 6. SQLite Rules (Canonical Authoritative Database)

- All schema changes use migrations under `/migrations` (SQLite `STRICT` tables).
- Use foreign keys (`PRAGMA foreign_keys=ON` per connection).
- Use WAL mode (`journal_mode=WAL`) and `busy_timeout=5000` via `crates/infrastructure/src/db.rs`.
- Use unique constraints for graph relation identity and wiki page identity.
- Prefer recursive CTEs for bounded dependency traversal.
- Use atomic transactions (`BEGIN IMMEDIATE`) for all authoritative graph mutations.
- Never issue per-node SQL queries for a large traversal when one bounded recursive CTE query suffices.
- Use `TEXT` ISO-8601 (`strftime('%Y-%m-%dT%H:%M:%fZ','now')`) for timestamps.
- Store probabilities as `REAL` with `CHECK (x >= 0.0 AND x <= 1.0)`.
- Store structured JSON as `TEXT CHECK (json_valid(...))`.

---

## 7. API & Streaming Rules

### Endpoints
Knowledgeable provides:
1. LibreChat Adapter routes (`/api/*`):
   - `GET /api/config`
   - `GET /api/user`
   - `POST /api/auth/refresh`
   - `GET /api/convos`
   - `GET /api/messages/:conversationId`
   - `POST /api/ask/knowledgeable` (or `/api/agents/chat/knowledgeable`)
2. Knowledgeable Extension routes:
   - `GET /api/concepts/:id/wiki`
   - `GET /api/graph/neighborhood`

### Server-Sent Events (SSE) Protocol
- Stream must emit compliant LibreChat payloads:
  - `data: {"created": true, "message": {...}}`
  - `data: {"text": "..."}` (content deltas)
  - `data: {"event": "concept_annotations", "data": {"concepts": [...]}}` (semantic annotations)
  - `data: {"final": true, "conversation": {...}, "message": {...}}`
- Stream termination must be explicit.

---

## 8. LLM Abstraction & Provider Rules

- Only `crates/llm` may depend on LLM provider APIs.
- Provider adapters translate provider-specific chunks, tool calls, and errors into domain types.
- Supports BYOK (Bring Your Own Key) and local models (Ollama/vLLM) via standard OpenAI-compatible endpoints.
- Consumer subscription hijacking is prohibited.
