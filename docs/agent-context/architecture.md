# Knowledgeable — Architecture

## 1. Scope

<architecture_goal>
Build an AI tutor whose persistent learner graph models what an individual understands and is actively used to teach at the edge of that learner's understanding, presented through a battle-tested, rich conversational interface derived from LibreChat.
</architecture_goal>

## 2. Product Model

<product_definition>
Knowledgeable is not a generic chatbot and not an abstract graph visualizer.
The primary interaction is natural conversational tutoring.
The graph is the persistent learner model that makes each future explanation grounded and adapted to the learner's frontier.
The personal wiki is a persistent, human-readable projection of that graph.
</product_definition>

### Core Loop

<core_loop>
1. Learner asks a question or explores a topic.
2. Tutor resolves target concepts in the learner graph.
3. Tutor proactively queries the graph (ADR-003) to inspect the learner's frontier and prerequisite health.
4. Tutor explains using established concepts as anchors, inserting missing foundations as needed.
5. Missing concepts or relationships are proposed as candidates with `world_confidence` validation.
6. Tutor observes evidence of learner understanding or confusion.
7. Learner confidence updates are committed transactionally to SQLite.
8. Concepts mentioned in the explanation are semantically highlighted in the chat UI based on learner confidence.
9. Mastered concepts generate or update persistent Personal Knowledge Wiki pages.
10. Accepted graph additions expand the learner's foundation for future sessions.
</core_loop>

---

## 3. Architectural Invariants

<system_constraints>
- **Presentation Layer**: Vendored LibreChat client (`apps/web`) is the primary presentation layer. The legacy Flutter client (`apps/client`) is deprecated.
- **Backend Core**: The backend is Rust (Axum, Tokio) and owns all authoritative state, graph logic, and AI orchestration.
- **Single Canonical Database**: SQLite is the canonical authoritative persistent store (`knowledgeable.db`, `journal_mode=WAL`, `foreign_keys=ON`, `busy_timeout=5000`, STRICT tables, atomic transactions).
- **Zero-Daemon Topology**: No MongoDB, Postgres, Redis, MeiliSearch, or Docker containers are required for default local execution.
- **Authoritative Graph Rule**: The graph is persistent domain state in SQLite, not chat history. Canonical knowledge is structured; the LLM is not the database.
- **Dual Confidence Model**:
  - `world_confidence` (>= 0.80) gates authoritative admission of concepts/relations into the global knowledge pool.
  - `learner_confidence` (0.0 to 1.0) measures the individual's understanding, subject to time-based decay with a 2-year grace period.
- **Proactive Graph Navigation (ADR-003)**: The tutor orchestrator proactively queries graph tools *before* and *during* explanation. The tutor never receives an unconstrained graph prompt dump.
- **Clean Markdown Protocol**: The LLM outputs standard, unpolluted Markdown. Concept highlighting is applied post-generation via AST inspection against the graph, never through brittle LLM-generated HTML.
- **Persistent Wiki as Derived Projection**: The Personal Knowledge Wiki is a persistent, cached artifact derived from the authoritative graph. It is maintained with low frequency upon milestone transitions, never regenerated on every visit.
- **Strict Separation of Auth**: Learner identity (local profile or account) is decoupled from Model provider authorization (BYOK developer keys, server env keys, or local Ollama/vLLM endpoints). Consumer subscriptions (ChatGPT Plus, Gemini Advanced) are not hijackable and require official API keys.
</system_constraints>

---

## 4. System Topology

```text
               ┌─────────────────────────────────────────┐
               │         LibreChat Client (React/Vite)   │
               │                   apps/web              │
               │  - Chat UX, Streaming, Markdown/LaTeX   │
               │  - Concept Highlighting (Remark plugin) │
               │  - Personal Wiki Drawer                 │
               │  - Graph Explorer Drawer                │
               └────────────────────┬────────────────────┘
                                    │ HTTP / SSE (/api/*)
                                    ▼
               ┌─────────────────────────────────────────┐
               │         Knowledgeable Axum Server       │
               │                 crates/api              │
               │  - LibreChat API Adapter (/api/*)       │
               │  - Native REST API (/v1/*)              │
               └────────────────────┬────────────────────┘
                                    │
                                    ▼
               ┌─────────────────────────────────────────┐
               │             Application Layer           │
               │            crates/application           │
               │  - Tutor Orchestration                  │
               │  - Conversation Management              │
               │  - Graph Query & Mutation Engine        │
               │  - Wiki Caching & Staleness Engine      │
               └─────────┬──────────────────────┬────────┘
                         │                      │
       Domain Ports      │                      │ Provider-neutral
                         ▼                      ▼
               ┌──────────────────┐   ┌──────────────────────────┐
               │    crates/domain │   │        crates/llm        │
               │  - Concepts      │   │  - Streaming LlmClient   │
               │  - Relations     │   │  - Gemini / OpenAI       │
               │  - Learner State │   │  - Anthropic / Local     │
               │  - Invariants    │   └──────────────────────────┘
               │  - Wiki Models   │
               └─────────┬────────┘
                         │ SQLx
                         ▼
               ┌─────────────────────────────────────────┐
               │       SQLite (knowledgeable.db)         │
               │  WAL mode, foreign_keys=ON, STRICT      │
               │  - Conversations & Messages             │
               │  - Concept Nodes & Relations            │
               │  - Learner Concept States               │
               │  - Observations & Candidates            │
               │  - Concept Wiki Pages (cached)          │
               └─────────────────────────────────────────┘
```

---

## 5. Monorepo Structure

```text
/
├── apps/
│   ├── web/                            # Vendored LibreChat workspace (React, Vite, Tailwind, TanStack Query)
│   │   ├── client/                     # @librechat/frontend SPA
│   │   │   └── src/
│   │   │       ├── knowledgeable/      # Knowledgeable extensions (Wiki drawer, Concept renderer, Graph view)
│   │   │       └── ...                 # Upstream client components, hooks, stores
│   │   ├── packages/
│   │   │   ├── client/                 # @librechat/client component library
│   │   │   ├── data-provider/          # librechat-data-provider (API client, schemas, query hooks)
│   │   │   └── data-schemas/           # @librechat/data-schemas
│   │   └── package.json                # Workspace root (proxies /api to Axum on :3000)
│   └── client/                         # [DEPRECATED] Legacy Flutter client
├── crates/
│   ├── api/                            # Axum HTTP/SSE server + LibreChat adapter (/api/*)
│   ├── application/                    # Tutor loop, conversation service, graph service, wiki service
│   ├── tutor/                          # System prompts, proactive query protocol, tool definitions
│   ├── domain/                         # Pure domain entities, invariants, decay math, repository traits
│   ├── llm/                            # Provider-neutral LLM client traits + provider implementations
│   └── infrastructure/                 # SQLite repositories (SQLx), WAL connection pool, migrations
├── migrations/                         # SQLite migrations (STRICT schema, TEXT UUIDs, ISO8601 UTC)
├── docs/
│   └── agent-context/
│       ├── architecture.md             # This document
│       ├── integration/librechat.md    # LibreChat seam analysis, contracts, upgrade plan
│       ├── data_models.md              # Authoritative domain & persistence models
│       ├── tech_stack_and_rules.md     # Development guidelines & architectural rules
│       ├── prompt_guidelines.md        # Agent behavior rules
│       └── roadmap_and_state.md        # Dependency-ordered milestone roadmap
├── tests/                              # Integration & E2E tests
├── Cargo.toml                          # Cargo workspace definition
└── knowledgeable.db                    # File-local SQLite database
```

---

## 6. Component Boundaries & Responsibilities

### 6.1 Presentation Layer: `apps/web` (LibreChat Client)
- **Role**: Presentation and interaction only.
- **Responsibilities**:
  - Message rendering (Markdown, KaTeX math formulas, syntax highlighted code, tables).
  - Streaming SSE consumption via `useSSE`.
  - Concept highlighting: Injects `remarkConcept` plugin into `react-markdown` pipeline to render known/weak/new concepts with confidence badges.
  - Slide-over drawers for Personal Knowledge Wiki and Knowledge Graph explorer.
  - Settings UI for model selection and user BYOK API keys.
- **Forbidden**:
  - Direct calculation of learner confidence or graph decay.
  - Direct mutation of authoritative graph state.
  - Direct LLM provider network calls bypassing the backend tutor orchestrator.

### 6.2 API & Adapter Layer: `crates/api`
- **Role**: Transport and protocol adapter.
- **Responsibilities**:
  - Maps LibreChat client endpoints (`/api/config`, `/api/user`, `/api/convos`, `/api/messages`, `/api/ask`) to internal application services.
  - Formats tutor stream events into LibreChat SSE event contracts (`created`, delta chunks, tool steps, `concept_annotations`, `final`).
  - Exposes native Knowledgeable endpoints (`/api/concepts/:id/wiki`, `/api/graph/neighborhood`).
  - Enforces request validation and maps internal domain errors to HTTP statuses.

### 6.3 Application Layer: `crates/application`
- **Role**: Use case orchestration and workflow execution.
- **Responsibilities**:
  - `TutorService`: Drives the multi-turn conversational loop, invokes the LLM, executes proactive graph tools, and crystallizes turn evidence.
  - `ConversationService`: Manages conversation lifecycles and message histories in SQLite.
  - `GraphService`: Executes bounded neighborhood queries, prerequisite checks, and transactional mutations.
  - `WikiService`: Manages personal wiki page generation, caching, and staleness evaluation.

### 6.4 Tutor & Prompt Layer: `crates/tutor`
- **Role**: Pedagogical intelligence and tool definitions.
- **Responsibilities**:
  - Prompts enforcing ADR-003 Proactive Graph Querying protocol.
  - Typed tool definitions: `find_concept`, `get_concept`, `get_dependencies`, `get_weak_dependencies`, `propose_concept`, `log_observation`.
  - Tool execution handlers executing against domain repository ports.

### 6.5 Domain Layer: `crates/domain`
- **Role**: Pure business logic and domain invariants (Zero I/O).
- **Responsibilities**:
  - Entities: `ConceptNode`, `ConceptRelation`, `LearnerConceptState`, `LearnerObservation`, `ConceptCandidate`, `ConceptWikiPage`.
  - Invariants: Admission gates (`world_confidence >= 0.80`), confidence clamps `[0.0, 1.0]`, DAG cycle prevention.
  - Half-life confidence decay calculations.
  - Repository interfaces (ports).

### 6.6 Infrastructure Layer: `crates/infrastructure`
- **Role**: Concrete persistence and external integrations.
- **Responsibilities**:
  - SQLite repositories implemented with SQLx using recursive Common Table Expressions (CTEs) for graph traversal.
  - Connection pooling with `journal_mode=WAL`, `foreign_keys=ON`, and `busy_timeout=5000`.
  - Atomic database transactions for graph updates.

---

## 7. Knowledge Triad: Graph, Annotations, and Wiki

Authoritative knowledge flows through three interconnected projections:

```text
                       AUTHORITATIVE GRAPH
                      (SQLite Persistent DAG)
                                 │
                 ┌───────────────┼───────────────┐
                 │               │               │
                 ▼               ▼               ▼
           Tutor Context   UI Annotations   Personal Wiki
          (Bounded Tool      (Real-time      (Persistent
           Retrieval)       AST Highlighting) Cached Artifact)
```

1. **Authoritative Graph**: Single source of truth in SQLite. Represents concepts, semantic relations, prerequisite dependencies, and learner confidence.
2. **Tutor Context**: Dynamically assembled candidate frontier retrieved by bounded tools during a tutoring turn.
3. **UI Annotations**: Real-time semantic highlights matching message text against known concepts, color-coded by learner health (known = subtle, weak = amber, new = blue).
4. **Personal Wiki**: Persistent human-readable markdown explanations anchored to the learner's existing prerequisites. Cached in SQLite, marked stale on graph mutations, and regenerated with low frequency.

---

## 8. Deprecation and Migration Strategy

The legacy Flutter client (`apps/client`) is formally deprecated:
- **Obsolete Components**: Flutter Riverpod state providers, Flyer Chat widget wrappers, Drift SQLite client database, and Flutter Web/Android/iOS build pipelines.
- **Retained & Reused Concepts**: Graph neighborhood traversal visualization logic, review queue contracts, and theme tokens are translated into React components within `apps/web/client/src/knowledgeable/`.
- **Clean Transition**: Development focuses 100% on the `apps/web` (LibreChat client) + `crates/` (Rust core) architecture.
