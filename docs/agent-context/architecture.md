# Knowledgeable — Architecture

## 1. Scope

<architecture_goal>
Build a cross-platform AI tutor whose persistent learner graph models what a person understands and is actively used to teach at the edge of that learner's understanding.
</architecture_goal>

## 2. Product Model

<product_definition>
Knowledgeable is not a graph-management product and not a generic chatbot.
The primary loop is conversational tutoring.
The graph is the persistent learner model that makes each future explanation more relevant.
</product_definition>

### Core loop

<core_loop>
1. Learner asks to understand something.
2. Tutor resolves the target concept when possible.
3. Tutor queries the learner graph through bounded tools.
4. Tutor identifies the current learning frontier.
5. Tutor teaches using strong known concepts as anchors and inserts only necessary missing concepts.
6. If required knowledge is missing, tutor proposes candidate concepts and dependencies.
7. Candidate knowledge passes validation before authoritative graph admission.
8. Tutor observes evidence about learner understanding.
9. Learner confidence is updated.
10. Concepts below the health threshold become review-eligible.
11. Accepted concepts and relations expand the learner graph for future sessions.
</core_loop>

## 3. Architectural Invariants

<system_constraints>
- Flutter is the canonical client technology for mobile and web.
- The backend is Rust and owns authoritative application state and AI orchestration.
- SQLite is the canonical authoritative persistent store for graph/session/learner state (WAL, foreign_keys=ON, busy_timeout=5000, transactions for mutations).
- PostgreSQL is NOT part of default dev/deploy; SQLite file is created automatically via sqlx `sqlite:knowledgeable.db` with `create_if_missing(true)`.
- SQLite on the client (Drift) is a local cache/offline/read-optimized copy, never a competing source of truth.
- Docker is NOT required for local development; native binaries + SQLite file is the canonical path.
- The learner graph is persistent domain state, not chat history and not the primary UI.
- Canonical knowledge is structured; the LLM is not the database.
- `world_confidence` gates authoritative graph admission.
- `learner_confidence` measures learner understanding/health.
- Canonical truth is stable; learner-facing wording is adaptive.
- Semantic/reference edges and dependency edges are distinct.
- The tutor teaches from the learner frontier by default.
- Weak prerequisites are repaired before heavily depending on them.
- Missing knowledge is created on demand; do not require a giant universal ontology.
- LLM output is untrusted input until deterministic validation and authorization succeed.
- Clients cannot directly mutate authoritative graph knowledge.
- Tutor graph navigation is tool-driven and bounded; never dump the entire graph into an LLM prompt.
- Keep the initial domain model intentionally small.
</system_constraints>

## 4. System Topology

```text
                         ┌─────────────────────────┐
                         │        Flutter          │
                         │                         │
                         │ Chat                    │
                         │ Sessions                │
                         │ Review                  │
                         │ Graph inspection        │
                         │ Local SQLite cache      │
                         └────────────┬────────────┘
                                      │ HTTPS + SSE
                                      ▼
                         ┌─────────────────────────┐
                         │       Rust API          │
                         │         Axum            │
                         │                         │
                         │ Auth / API              │
                         │ Tutor orchestration     │
                         │ Graph application svc   │
                         │ Learner state service   │
                         │ Validation              │
                         └─────┬──────────┬────────┘
                               │          │
                      SQLx      │          │ provider-neutral LLM client
                                ▼          ▼
                      ┌──────────────┐  ┌───────────────────┐
                      │   SQLite     │  │ LLM providers     │
                      │ authoritative│  │ model adapters    │
                      │ WAL + FKs    │  │                   │
                      └──────────────┘  └───────────────────┘
```

## 5. Monorepo Structure

```text
/
├── apps/
│   └── client/                         # Flutter application: mobile + web
│       ├── lib/
│       │   ├── app/                    # app bootstrap, routing, theme
│       │   ├── core/                   # shared client infrastructure
│       │   ├── features/
│       │   │   ├── auth/
│       │   │   ├── chat/
│       │   │   ├── sessions/
│       │   │   ├── review/
│       │   │   └── graph/
│       │   ├── data/                   # API/local repositories, DTOs
│       │   ├── state/                  # Riverpod providers/notifiers
│       │   └── widgets/                # reusable UI primitives
│       ├── test/
│       ├── web/
│       ├── android/
│       └── ios/
├── crates/
│   ├── api/                            # Axum routes/controllers
│   ├── application/                    # use cases/orchestration (depends on domain ports)
│   ├── domain/                         # pure domain types + invariants + validation + decay + graph ports
│   ├── tutor/                          # tutor loop, tool contracts, prompts
│   ├── llm/                            # provider-neutral LLM traits + adapters
│   └── infrastructure/                 # SQLx SQLite (WAL/FKs/busy_timeout), auth, telemetry, config
├── migrations/                         # SQLite migrations (STRICT, TEXT UUIDs, ISO8601)
├── docs/
│   └── agent-context/
├── tests/
│   ├── integration/
│   └── e2e/
├── scripts/
├── Cargo.toml
└── pubspec.yaml / apps/client/pubspec.yaml
```

Cross-references:
- Data names and persistence schemas: `data_models.md`.
- Framework/library rules: `tech_stack_and_rules.md`.
- Implementation order: `roadmap_and_state.md`.
- Agent behavior: `prompt_guidelines.md`.

## 6. Module Boundaries

### 6.1 Flutter client: `apps/client`

Responsibilities:
- Chat UI and streaming rendering.
- Session list/history.
- Review suggestions and review interaction.
- Optional graph topology inspection.
- Local SQLite caching.
- Client-side navigation and transient UI state.
- Syncing server-authoritative state into local storage.

Must not:
- Decide truth/world confidence.
- Calculate authoritative learner confidence.
- Mutate graph nodes/relations directly.
- Call LLM providers directly.
- Encode pedagogical business rules that belong to the backend.

See `tech_stack_and_rules.md -> Flutter Rules` and `data_models.md -> API Models`.

### 6.2 API: `crates/api`

Responsibilities:
- HTTP/SSE endpoints.
- Authentication and authorization boundary.
- Request/response serialization.
- Request validation.
- Rate limiting hooks.
- Mapping application errors to stable API errors.

Must remain thin; business logic belongs in `crates/application` and lower layers.

### 6.3 Application: `crates/application`

Responsibilities:
- Coordinate use cases spanning multiple domain services.
- Own transaction boundaries.
- Enforce authorization context before service calls.
- Coordinate tutor, graph, learner, validation, and persistence.

Examples:
```text
start_session
send_message
stream_tutor_turn
commit_graph_mutation
create_review_session
apply_learner_observations
```

### 6.4 Domain: `crates/domain`

Pure logic only — database-agnostic, no I/O.

Responsibilities:
- IDs and value objects.
- Concept and relation invariants.
- Confidence bounds (`world_confidence`, `learner_confidence`).
- Validation (schema, `world_confidence >= 0.80` gate, canonical statement checks).
- Learner confidence: initialization, bounded updates, simple time-based decay (`grace 2y`, `half-life 12y`), review eligibility (`<0.95`).
- Graph ports & traversal: `GraphRepository` trait (bounded, learner-scoped), `bounded_depth`, `TutorContext` types, `DEFAULT_MAX_DEPTH=3`.
- State transitions and domain-level errors.

Core query (implemented in `infrastructure` via SQLite recursive CTEs, defined as port in `domain`):

```text
retrieve_learning_context(target, learner)
    -> resolve target
    -> retrieve target concept
    -> traverse dependency ancestors to bounded depth
    -> retrieve bounded semantic neighbors
    -> retrieve weak prerequisites
    -> include learner confidence for returned concepts
    -> return TutorContext
```

Domain does not decide the complete pedagogical sequence — the tutor does. Domain stays independent of Axum, SQLx, HTTP, Flutter, and LLM providers. PostgreSQL can be introduced later by implementing the same ports without rewriting domain/application.

Forbidden dependencies:
- Axum
- SQLx
- LLM SDKs
- Flutter/client code
- filesystem/network I/O

### 6.5 Tutor: `crates/tutor`

Responsibilities:
- Tutor system behavior.
- Tool schemas.
- LLM tool loop.
- Target/frontier reasoning.
- Graph navigation decisions.
- Candidate concept/relation proposals.
- Extraction of structured learner observations.
- Tutor event streaming.

The tutor may read graph state dynamically but cannot bypass application authorization or validation.

### 6.6 LLM: `crates/llm`

Responsibilities:
- Provider-neutral traits.
- Streaming text.
- Structured generation.
- Tool-call transport.
- Provider adapters.
- Retry policy for transient provider errors.
- Provider error normalization.

No LLM vendor types may escape this crate's public abstraction.

### 6.7 Infrastructure: `crates/infrastructure`

Responsibilities:
- SQLx/SQLite (WAL, foreign_keys=ON, busy_timeout=5000, `sqlite:knowledgeable.db` with `create_if_missing`).
- SQLite pragmas per connection, transactions for graph mutations (`BEGIN IMMEDIATE` where needed).
- Authentication adapter.
- Configuration (`DATABASE_URL` defaults to `sqlite:knowledgeable.db`).
- Telemetry/logging.
- External service adapters.

Infrastructure implements `domain::GraphRepository` and other ports; application depends on domain ports, not infrastructure concretions. PostgreSQL can be reintroduced later by implementing the same ports.

## 7. Tutor Graph-Navigation Model

The tutor operates over two spaces:

```text
LEARNER GRAPH
    = what the learner already has + confidence/health

CONCEPTUAL SEARCH SPACE
    = concepts the tutor may need to introduce
```

### Known territory

```text
user request
    -> target resolved
    -> bounded graph reads
    -> strong known concepts identified
    -> weak prerequisites identified
    -> tutor teaches from the frontier
```

### Unknown territory

```text
user request
    -> target absent/poorly connected
    -> tutor asks graph for nearby known foundations
    -> tutor proposes missing intermediate concepts
    -> candidates validated
    -> accepted concepts become graph state
    -> tutor continues the lesson
```

### Tutor tool boundary

Minimum tool set:

```text
find_concept(query, limit)
get_concept(concept_id)
get_dependencies(concept_id, depth)
get_related_concepts(concept_id, limit)
get_learner_confidence(concept_ids)
get_weak_dependencies(concept_id, threshold, depth)
propose_concept(candidate)
propose_relation(candidate)
propose_learner_update(update)
```

The exact serialized contracts are defined in `data_models.md -> Tutor Tool Contracts`.

## 8. Dynamic Context Construction

<dynamic_context_rule>
Do not attempt to solve pedagogical context selection with either a giant prompt or a purely deterministic algorithm.
Use deterministic bounded retrieval to construct a candidate region; let the tutor LLM decide which pieces matter and request additional graph reads when necessary.
</dynamic_context_rule>

Pipeline:

```text
Target request
    ↓
Target resolution
    ↓
Deterministic bounded retrieval
    ↓
Candidate TutorContext
    ↓
LLM reasoning + graph tool calls
    ↓
Teaching sequence / frontier decision
```

The system must support an initially sparse graph. Missing intermediate concepts are normal and are created incrementally.

## 9. Graph Mutation Pipeline

<graph_mutation_pipeline>
Conversation
    ↓
Tutor reasoning
    ↓
Candidate concept/relation/learner-update proposals
    ↓
Typed schema validation
    ↓
Identity/duplicate resolution
    ↓
World-confidence admission gate
    ↓
Relation/invariant validation
    ↓
Transactional commit
    ↓
Learner confidence initialization/update
    ↓
Persist mutation audit record
</graph_mutation_pipeline>

Admission rule:

```text
world_confidence < WORLD_CONFIDENCE_MIN
    -> authoritative node admission rejected

world_confidence >= WORLD_CONFIDENCE_MIN
    -> candidate may be admitted if all other validation passes
```

Initial `WORLD_CONFIDENCE_MIN = 0.80`. See `data_models.md -> Constants`.

## 10. Learner Graph Repair

When a learner's understanding is found to be weaker than expected:

```text
current target
    ↓
weak required prerequisite
    ↓
inspect dependency ancestors
    ↓
identify smallest weak path
    ↓
repair/reteach
    ↓
observe learner understanding
    ↓
update learner_confidence
    ↓
return to original target
```

Do not review unrelated graph regions.

## 11. Persistence

SQLite is the canonical authoritative backend store (file-local, no daemon, no Docker).

```text
Rust backend
  ↔ SQLx SQLite (WAL + foreign_keys + busy_timeout)
  ↔ knowledgeable.db (auto-created, STRICT tables, TEXT UUIDs, ISO8601)
```

Do not introduce a dedicated graph database in v1. Do not add PostgreSQL, pgvector, Redis, queues, or vector DBs unless a concrete roadmap requirement justifies it. SQLite's recursive CTEs handle bounded dependency traversal; transactions guarantee atomic mutations. SQLite can be replaced by PostgreSQL later via the same domain ports without rewriting domain/application.

Client SQLite (Drift) is a cache/local mirror only — server remains authoritative for canonical graph, mutations, and learner state.

## 12. Client Synchronization

```text
Flutter action
    -> API request
    -> server transaction
    -> server response / SSE event
    -> local repository update
    -> UI update
```

Rules:
- Server state wins on conflicts.
- Local writes may be optimistic only for UI-only state.
- Graph/learner authoritative mutations are server-confirmed before final local commit.
- Sync payloads are versionable and typed.

## 13. Runtime Request Flow

```text
Flutter
  -> POST /v1/conversations/:conversation_id/messages
Rust API
  -> application::stream_tutor_turn
Tutor
  -> graph tools (domain ports)
Graph (domain port)
  -> SQLite (WAL, FKs) via sqlx
Tutor
  -> LLM abstraction
LLM provider
  -> streamed events
Tutor
  -> SSE events to client
Tutor
  -> structured post-turn observation/candidate extraction
Application
  -> domain validation (world_confidence >=0.80) + transaction
SQLite
  -> authoritative mutation (BEGIN IMMEDIATE, atomic)
Flutter
  -> apply confirmed events to local SQLite (Drift cache)
```

## 14. Security Boundary

- All graph access is scoped by authenticated `learner_id`.
- The model never supplies the authoritative authenticated learner identity.
- Tool implementations derive learner scope from server context.
- Secrets exist only server-side.
- Client SQLite stores only data permitted for local caching.
- Candidate graph writes occur only through server application services.
- LLM output is untrusted.
- User content is untrusted.

## 15. Observability

Capture structured metadata for:
- request/session/turn IDs
- model/provider identifiers
- tool calls and tool latency
- graph retrieval depth/count
- candidate proposal counts
- validation rejection reasons
- learner-confidence changes
- token/cost metrics when available
- end-to-end latency
- database query latency

Do not log full private conversation content by default.

## 16. Testing Boundaries

Unit tests:
- domain invariants
- confidence updates
- decay
- graph traversal
- validation
- tutor decision helpers

Integration tests:
- Axum API + SQLite (in-memory or `sqlite:file:memdb?mode=memory&cache=shared` for isolation)
- graph repository (SQLite recursive CTEs, bounded depth)
- tutor tools
- transactional graph mutation (WAL + FKs, rollback on validation failure)
- SSE streaming
- authentication scope

Client tests:
- widgets
- Riverpod state
- repository mapping
- stream event handling

E2E tests:
- empty graph -> first learning session -> graph growth
- known concept -> personalized explanation
- weak prerequisite -> repair -> continuation
- low world confidence -> candidate rejected
- long-term decay -> review suggestion

See `roadmap_and_state.md -> Verification Gates`.
