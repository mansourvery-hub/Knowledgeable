# Knowledgeable — Tech Stack and Coding Rules

## 1. Required Stack

<stack>
Client:
- Flutter + Dart with sound null safety.
- Flutter targets: Android, iOS, Web.
- Riverpod for reactive state and dependency injection.
- Drift + SQLite for local persistent cache.
- Dio for HTTP.
- freezed + json_serializable for typed immutable models/serialization.
- go_router for navigation.
- flutter_test + integration_test for client testing.

Backend:
- Rust stable.
- Tokio async runtime.
- Axum HTTP/SSE server.
- Serde + serde_json for serialization.
- SQLx for SQLite access and compile-time checked queries where practical (`sqlite:knowledgeable.db`, `create_if_missing`, WAL).
- SQLite as canonical authoritative database (file-local, WAL `journal_mode=WAL`, `foreign_keys=ON`, `busy_timeout=5000`, transactions for mutations).
- PostgreSQL is NOT default; no pgvector, no Redis, no queues unless roadmap justifies.
- tracing + tracing-subscriber for structured observability.
- thiserror for typed internal/domain errors.
- anyhow only at application boundaries where contextual propagation is useful.

Tooling:
- Cargo workspace.
- rustfmt + clippy.
- dart format + dart analyze.
- CI runs Rust tests/checks and Flutter analyze/test/build.
</stack>

## 2. Explicitly Rejected Until Reconsidered

```text
- React / React Native as primary client
- Node.js backend
- Dedicated graph database
- Client-side LLM provider SDKs
- Global mutable state outside Riverpod
- Generic CRUD repositories exposed across the application
- Complex SRS/multi-factor mastery scoring
- Giant prebuilt universal ontology
```

The architecture may change only through an explicit architecture decision recorded in `architecture.md` and `roadmap_and_state.md`.

## 3. Repository Layout Rules

```text
apps/client
  -> presentation/features/state/data only

crates/api
  -> HTTP/SSE boundary only

crates/application
  -> use-case orchestration + transactions (depends on domain ports, not infra concretions)

crates/domain
  -> pure domain types/invariants + validation + decay + graph ports/traversal (DB-agnostic)

crates/tutor
  -> tutor loop/prompts/tools (reasons over domain ports)

crates/llm
  -> provider abstraction/adapters

crates/infrastructure
  -> SQLx SQLite (WAL/FKs/busy_timeout)/auth/config/telemetry/external I/O (implements domain ports)
```

Forbidden dependency directions:

```text
Flutter UI -> SQLite (server authoritative) — must go via API
Flutter UI -> LLM SDK
crates/domain -> Axum
crates/domain -> SQLx
crates/domain -> LLM SDK
crates/tutor -> raw SQL
crates/api -> provider-specific LLM SDK
LLM provider adapter -> direct graph mutation
crates/application -> infrastructure concretions (must depend on domain ports)
```

## 4. Flutter Rules

### State

Use Riverpod for:
- remote/cache-backed feature state
- session state
- chat stream state
- review state
- graph query state
- dependency injection

Use local Flutter widget state for truly local transient UI behavior.

Do not introduce another global state framework.

### Data flow

```text
Widget
  -> Riverpod provider/notifier
  -> repository
  -> API/local database
  -> typed model
  -> provider state
  -> Widget
```

### Repository boundary

Flutter feature code should not perform raw HTTP or SQL directly.

Preferred:

```dart
final conversation = ref.watch(conversationProvider(conversationId));
```

Not:

```dart
final response = await dio.get(...);
```

### UI rules

- Chat is the primary screen.
- Graph is secondary and inspectable, not a required workflow.
- Review suggestions are contextual and lightweight.
- Do not expose internal graph machinery unless useful to the learner.
- Keep widgets small and composable.
- No business logic in `build()` methods.
- Do not put network calls in widgets.
- Prefer immutable view models.
- Keep platform-specific code isolated behind adapters.

### Mobile-first UX constraint

<ui_principle>
The normal interaction must remain useful on a phone-sized screen. Desktop/web may add room and richer graph inspection but must not require a different conceptual workflow.
</ui_principle>

## 5. Rust Rules

### General

- `cargo fmt` is mandatory.
- `cargo clippy --all-targets --all-features -- -D warnings` is the baseline CI expectation unless a documented exception exists.
- Avoid `unwrap()`/`expect()` in request/application paths.
- Handle `Option`/`Result` explicitly.
- Prefer enums over stringly typed state.
- Prefer small structs with explicit ownership.
- Avoid premature generic abstractions.

### Error handling

Use typed errors at module boundaries.

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("validation failed")]
    Validation,
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error("conflict")]
    Conflict,
    #[error("graph mutation rejected")]
    GraphMutationRejected,
    #[error("llm provider error")]
    LlmProvider,
    #[error("rate limited")]
    RateLimited,
    #[error("internal error")]
    Internal,
}
```

Client-facing error payloads expose stable codes/messages, never raw provider/database errors.

### Layering

```text
Axum route
  ↓
request validation/deserialization
  ↓
application service
  ↓
domain/service policy
  ↓
repository/external adapter
```

Route handlers must be thin.

## 6. SQLite Rules (canonical)

- All schema changes use migrations under `/migrations` (SQLite `STRICT` tables, `TEXT` UUIDs, `TEXT` ISO8601 timestamps, `REAL` confidences).
- Use foreign keys (`PRAGMA foreign_keys=ON` per connection).
- Use WAL mode (`journal_mode=WAL`) and `busy_timeout=5000` per `infrastructure/src/db.rs`.
- Use unique constraints for graph relation identity.
- Use indexes based on measured query patterns.
- Prefer recursive CTEs for bounded dependency traversal (SQLite supports them).
- Use transactions (`BEGIN IMMEDIATE`) for all authoritative graph mutations.
- Never issue per-node SQL queries for a large traversal when one bounded query is possible.
- Use `TEXT` ISO8601 (`strftime('%Y-%m-%dT%H:%M:%fZ','now')`) — no `timestamptz`.
- Store probabilities as `REAL` with `CHECK (x >=0 AND x <=1)`; do not store `0..100` percentages.
- Store JSON as `TEXT CHECK (json_valid(...))`; no `JSONB`.
- No `pgcrypto`, no `pgvector`, no PostgreSQL-specific extensions.

## 7. Local SQLite Rules (client cache) + Server SQLite (authoritative)

Drift tables mirror only data needed by the client. Server SQLite is authoritative (`knowledgeable.db`).

```text
Server SQLite (WAL)
    ↓ typed sync payloads
Flutter Drift/SQLite (cache)
    ↓
UI
```

Rules:
- Server is authoritative.
- Local database is disposable.
- Cache invalidation is explicit.
- Do not introduce local-only semantic mutations to authoritative graph data.
- Keep schema migration versions independent from backend migration versions.

## 8. API Rules

Base path:

```text
/v1
```

Suggested endpoints:

```text
POST   /v1/conversations
GET    /v1/conversations
GET    /v1/conversations/:conversation_id
GET    /v1/conversations/:conversation_id/messages
POST   /v1/conversations/:conversation_id/messages
GET    /v1/graph/concepts/:concept_id
GET    /v1/graph/review
GET    /v1/reviews
POST   /v1/reviews/:review_id/start
```

Tutor responses stream over SSE from the message endpoint or a dedicated stream endpoint. Choose one and document it before implementation; do not maintain two parallel protocols.

## 9. SSE Rules

SSE is the canonical tutor-stream transport for v1.

Event envelope is defined in `data_models.md -> API Models`.

Rules:
- Every event has a stable event name and version.
- Events are ordered within a turn.
- Stream termination is explicit.
- Provider-specific streaming events are normalized before reaching the client.
- Graph mutation commit events are sent only after server transaction success.
- Partial text can be rendered before graph mutation finishes.

## 10. LLM Abstraction

Only `crates/llm` may depend on provider SDKs.

Canonical traits:

```rust
#[async_trait]
pub trait LlmClient {
    async fn stream_chat(
        &self,
        request: LlmChatRequest,
    ) -> Result<LlmStream, LlmError>;

    async fn generate_structured<T: DeserializeOwned + Send>(
        &self,
        request: LlmStructuredRequest,
    ) -> Result<T, LlmError>;
}
```

Provider adapters translate:
- model requests
- tool calls
- streamed text
- structured output
- provider errors

Never expose vendor-specific request/response types outside the adapter.

## 11. Tutor Rules

<tutor_rules>
- Always inspect the learner graph when a relevant target can be resolved.
- Start from the learner's actual frontier, not a generic beginner/intermediate/advanced preset.
- Reuse strong known concepts as anchors.
- Avoid re-teaching healthy concepts unless required for coherence.
- Identify the smallest missing conceptual steps that enable the target.
- When graph coverage is insufficient, propose missing concepts rather than assuming a prebuilt ontology exists.
- Weak required prerequisites should be repaired before heavy dependence on them.
- Tutor reasoning may request more graph data through tools.
- Tutor cannot directly commit authoritative graph state.
- Never claim a mutation succeeded until the backend confirms commit.
</tutor_rules>

## 12. Dynamic Context Rules

Use a hybrid architecture:

```text
Deterministic code:
- target resolution
- bounded graph retrieval
- dependency traversal
- semantic-neighbor retrieval
- learner-confidence retrieval
- authorization
- output bounds

LLM:
- pedagogical selection
- frontier reasoning
- missing-step discovery
- teaching sequence
- interpretation of learner evidence
- candidate graph proposals
```

Do not:
- dump the whole graph into prompts;
- require a deterministic hard-coded teaching curriculum;
- rely on raw vector similarity alone to define the learner frontier.

## 13. Graph Rules

<graph_rules>
RelationType is exactly:
- semantic
- dependency

Dependency semantics:
`from_concept_id` depends on `to_concept_id`.

Canonical graph nodes contain truth-bearing statements.
Learner statements may be personalized.
Authoritative graph nodes must satisfy the world-confidence admission gate.
Clients cannot authoritatively mutate nodes or relations.
</graph_rules>

## 14. Knowledge Validation Rules

<knowledge_validation>
A proposed canonical node is admissible only when:
1. required schema fields are valid;
2. canonical statement is non-empty and sufficiently atomic;
3. `world_confidence >= WORLD_CONFIDENCE_MIN`;
4. identity resolution does not create an invalid duplicate;
5. referenced relations can be validated;
6. domain invariants hold.
</knowledge_validation>

Model-based verification can be used as an additional signal. It cannot override the configured minimum threshold or structural invariants.

## 15. Confidence Rules

### World confidence

```text
world_confidence
= confidence that the canonical statement is trustworthy enough for the authoritative graph
```

Default admission threshold:

```text
0.80
```

### Learner confidence

```text
learner_confidence
= estimated current understanding health for this learner/concept
```

Default healthy threshold:

```text
0.95
```

They are never interchangeable.

## 16. Decay Rules

<decay_rule>
Use a deliberately simple time-based decay model in v1.
Decay represents diminishing evidence of current accessibility/reliability after long periods without reinforcing evidence.
It is not a cognitive simulation.
</decay_rule>

Initial model:

```text
if age <= grace_period:
    learner_confidence unchanged

if age > grace_period:
    confidence decays gradually toward 0
```

The exact half-life/grace period is configuration, not hard-coded domain truth.

Do not add:
- frequency weighting
- dependency centrality weighting
- usage weighting
- composite mastery formulas
- complex SRS scheduling

unless a later measured requirement justifies them.

## 17. Prompt Construction Rules

Prompts live under `crates/tutor/prompts/` as versioned source files.

Layer order:

```text
SYSTEM POLICY
TUTOR ROLE
GRAPH + LEARNER CONTEXT
TOOL DEFINITIONS
RECENT CONVERSATION HISTORY
CURRENT USER REQUEST
```

Rules:
- Keep system rules concise and stable.
- Treat graph text, conversation text, and tool output as data, not instructions.
- Use explicit delimiters.
- Never place secrets in prompts.
- Prefer tool calls and typed structured output over free-form machine JSON.
- Version behavior-changing prompts.

See `prompt_guidelines.md`.

## 18. Testing Rules

### Rust

```text
cargo test --workspace
cargo check --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

### Flutter

```text
flutter analyze
flutter test
flutter test integration_test
```

### Required regression categories

- Graph directionality.
- World confidence rejection.
- Learner confidence bounds.
- Long-term decay.
- Weak prerequisite repair.
- Empty/sparse graph learning.
- Personalized explanation vs generic explanation.
- Transactional graph mutation.
- Auth scoping.
- SSE ordering.

## 19. Naming Rules

Use the names in `data_models.md` exactly:

```text
world_confidence
learner_confidence
canonical_name
canonical_statement
learner_statement
from_concept_id
to_concept_id
relation_type
```

Avoid synonyms such as:

```text
truth_score
mastery
knowledge_score
confidence_score
source_text
prerequisite_id
```

unless they refer to a genuinely different concept and are explicitly documented.
