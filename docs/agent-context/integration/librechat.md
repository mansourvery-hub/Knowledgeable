# Knowledgeable — LibreChat Integration Architecture

## 1. Executive Summary

Knowledgeable differentiates itself not through a custom chat UI, but through its **persistent learner model and frontier tutoring engine**. Rebuilding commodity chat capabilities (streaming, markdown, LaTeX, syntax highlighting, copy/export, message branching, mobile responsiveness) diverts engineering effort from our core value proposition.

We integrate **LibreChat** as the presentation layer. LibreChat provides a battle-tested, highly polished ChatGPT-grade UX. However, upstream LibreChat couples its Node.js backend to MongoDB, Redis, and MeiliSearch. Adopting that backend directly would violate Knowledgeable's foundational architectural invariant: **a zero-dependency, single-binary Rust application with a local SQLite database**.

Therefore, Knowledgeable adopts a **Vendored Frontend + Native Rust API Adapter** strategy (synthesizing Options C and D):
1. **Frontend (`apps/web`)**: Vendored LibreChat client (`client/`, `@librechat/client`, `@librechat/data-provider`), stripped of unused enterprise modules, configured with Vite to proxy `/api` requests directly to Knowledgeable's Rust backend.
2. **Backend Adapter (`crates/api/src/librechat/`)**: Knowledgeable's Axum server implements the precise subset of LibreChat REST endpoints and SSE protocols needed by the frontend, backed natively by SQLite and Knowledgeable's domain services.
3. **Domain Extensions**:
   - **Concept-Aware Chat Rendering**: Remark/Unist AST plugin (`remarkConceptHighlight`) annotating recognized learner concepts in assistant messages without corrupting raw markdown or requiring the LLM to emit brittle HTML.
   - **Personal Knowledge Wiki Drawer**: Dedicated UI side panel rendering persistent, cached concept wiki pages served from `/api/concepts/:id/wiki`.
   - **Knowledge Graph Explorer**: Interactive neighborhood visualizer embedded into the navigation, consuming `/v1/graph/neighborhood`.

---

## 2. Upstream LibreChat Architecture Audit

Inspection of current upstream LibreChat (`v0.8.8-rc3`) reveals the following workspace topology:

```text
LibreChat/
├── client/                     # @librechat/frontend (Vite + React 18 + Tailwind + Recoil + TanStack Query)
├── packages/
│   ├── client/                 # @librechat/client (UI component library, Ariakit/Radix primitives)
│   ├── data-provider/          # @librechat/data-provider (TypeScript API client, schemas, types, query hooks)
│   ├── data-schemas/           # Shared Zod schemas
│   └── api/                    # Node.js Express backend utilities & schemas
└── api/                        # Express server, MongoDB (Mongoose), Redis, MeiliSearch
```

### 2.1 Frontend Request Flow & Client Decoupling
- The frontend is a standard Vite Single Page Application (SPA).
- In development and production, the frontend routes all API calls through relative paths: `/api/*` and `/oauth/*`.
- In `client/vite.config.ts`, requests to `/api` are proxied to the configured backend URL (default `http://localhost:3080`).
- The entire client data-fetching layer is encapsulated in `@librechat/data-provider`, using `@tanstack/react-query` and standard `fetch`/`request.ts`.
- **Key Insight**: The frontend does not require the Node.js Express server if an alternative backend serves the expected HTTP/SSE endpoints.

### 2.2 Rendering Pipeline
The message rendering pipeline lives in `client/src/components/Chat/Messages/Content/`:
- `Markdown.tsx` splits messages into streaming blocks (`splitMarkdown.ts`).
- Blocks are passed to `react-markdown`, configured through `markdownConfig.ts`.
- The unified processing pipeline consists of:
  - Remark plugins: `remarkMath`, `remarkGfm`, `remarkDirective`, `artifactPlugin`, `mcpUIResourcePlugin`, `unicodeCitation`.
  - Rehype plugins: `rehypeKatex`, `rehypeHighlight`.
  - Custom markdown components: `getMarkdownComponents()` maps AST nodes (`code`, `a`, `p`, `table`, `artifact`, `citation`, `highlighted-text`) to rich React components.
- **Key Insight**: Concept-aware semantic highlighting can be cleanly inserted by adding a remark plugin to `markdownConfig.ts` and registering a custom component in `getMarkdownComponents()`. The LLM does not need to output HTML.

### 2.3 Streaming Protocol
LibreChat uses standard Server-Sent Events (SSE) via the `sse.js` client (`client/src/hooks/SSE/useSSE.ts`):
1. Client issues `POST` to the chat endpoint (e.g. `/api/ask/knowledgeable` or `/api/agents/chat/knowledgeable`) with a JSON payload (`TPayload`).
2. Server establishes an `event-stream` and emits:
   - Initial run/message creation:
     `data: {"created": true, "message": {"messageId": "...", "conversationId": "..."}}`
   - Content chunks (deltas):
     `data: {"text": "..."}`
   - Intermediate tool actions (optional):
     `data: {"event": "on_message_delta", ...}`
   - Final completion event:
     `data: {"final": true, "conversation": {"conversationId": "...", "title": "..."}, "message": {"messageId": "...", "content": "..."}}`
3. Connection closes cleanly.
- **Key Insight**: Knowledgeable's existing Rust SSE envelope in `crates/api/src/conversations.rs` can easily be adapted to format payloads matching these event shapes.

---

## 3. Comparison of Integration Options

| Evaluation Criterion | A. LibreChat Frontend + Knowledgeable API | B. LibreChat Agent + Knowledgeable Tools | C. LibreChat Frontend + Custom Axum Adapter | D. Vendored Frontend + Minimal Knowledgeable Patches | **E. Synthesized: Vendored Frontend (D) + Axum Adapter (C)** |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Implementation Complexity** | High (must reverse-engineer all frontend expectations without editing client) | High (requires full Node + Mongo + Redis + Rust tool server) | Moderate (Rust adapter translates protocols) | Moderate (maintaining frontend fork) | **Balanced & Manageable** |
| **Upstream Maintainability** | High for frontend, brittle at protocol boundary | High for LibreChat, poor for unified system | High (clean API contract) | Moderate (requires merge/rebase discipline) | **High (isolated patch surface + clear API boundary)** |
| **UX Quality** | Excellent (standard LibreChat) | Good (standard LibreChat) | Excellent | Excellent + Custom Knowledgeable Features | **Best-in-Class (LibreChat UX + Native Concept/Wiki UI)** |
| **Streaming Behavior** | Excellent (native SSE) | Good (agent streaming) | Excellent (Rust SSE) | Excellent | **Flawless (Rust tokio stream -> LibreChat useSSE)** |
| **Rich Rendering (Math/Code/Mermaid)**| Full support | Full support | Full support | Full support | **Full support + Custom Concept Highlighting** |
| **Auth & Provider Support** | Managed in Rust | Managed in Node/Mongo | Managed in Rust | Managed in Rust | **Clean separation: Rust owns local learner + BYOK** |
| **Pedagogical Tool Use (ADR-003)** | Guaranteed (Rust tutor orchestrator) | **Failed** (LLM skips tools unless coerced; state split) | Guaranteed (Rust tutor orchestrator) | Guaranteed | **Guaranteed (Rust owns proactive graph reasoning)** |
| **Conversation & Graph Persistence** | 100% SQLite | Split (Mongo for chat, SQLite for graph) | 100% SQLite | 100% SQLite | **100% SQLite (zero Docker, single file `knowledgeable.db`)** |
| **Concept Highlighting** | Impossible (cannot modify frontend bundle) | Difficult (requires raw text tags) | Impossible without client edit | Native via remark plugin | **Native, robust AST visitor via `remarkConcept`** |
| **Personal Wiki Integration** | Awkward (external browser window) | Awkward (external link) | Awkward without client UI | Native (integrated drawer/side-panel) | **Native slide-over drawer and linked reading view** |
| **Separation of Concerns** | Good | Poor (dual backend engines) | Excellent | Good | **Excellent: Web = UX, Rust = Domain/Tutor/Graph/State** |
| **Suitability for Agentic Coding** | Fair (black-box client) | Poor (multi-stack debugging across Node/Mongo/Rust) | Good | Good | **Optimal (well-scoped TypeScript UI + clean Rust crates)** |
| **Risk of Lock-in** | Low | High (locked into Mongo schema & agent runtime) | Low | Moderate | **Low (backend owns canonical schema; frontend is replaceable)** |

### 3.1 Why Option B is Rejected
Running LibreChat's Node.js backend and treating Knowledgeable as an "Agent Tool" violates the core product model:
- Knowledgeable requires **proactive tutor graph navigation** (ADR-003: the tutor inspects dependencies *before* choosing what to teach).
- Generic LLM agents call tools reactively and inconsistently.
- Storing conversation history in MongoDB while maintaining learner confidence in SQLite causes dual-source divergence.
- Requiring MongoDB, Node, Redis, and Rust violates the zero-external-dependency, file-local SQLite invariant.

### 3.2 Why Option A Alone is Insufficient
Consuming the frontend as an immutable npm package or untouched black-box bundle prevents implementing Knowledgeable's unique UI differentiators:
- **Concept highlighting** requires hooking into `react-markdown` via custom remark plugins in `markdownConfig.ts`.
- **Personal Wiki navigation** requires a dedicated slide-over panel or route in the React tree.
- **Graph neighborhood visualization** requires an integrated view.

### 3.3 The Selected Solution: Option E (Synthesized C + D)
- **Frontend (`apps/web`)**: Vendored LibreChat client. All non-essential upstream features (LDAP, Turnstile, multi-tenant balance tracking, third-party plugin marketplaces) are disabled via `startupConfig`. The codebase is trimmed to chat, conversation history, parameters/settings, and rich rendering.
- **Backend (`crates/api`)**: Axum implements the subset of `/api/*` endpoints required by LibreChat, forwarding requests into Knowledgeable's `application` services.
- **State**: The single SQLite database (`knowledgeable.db`) persists conversations, messages, concept nodes, relations, observations, candidates, and wiki pages.

---

## 4. API Specification: Knowledgeable LibreChat Adapter

The Rust backend implements the following `/api/*` endpoints under
`crates/api/src/routes/librechat/`. `knowledgeable` is the single advertised
endpoint key (`librechat::ENDPOINT_NAME`).

### 4.1 System & Session
- `GET /api/config`: Returns startup configuration (`TStartupConfig`). Local
  single-user mode disables auth surfaces (`emailLoginEnabled: false`,
  `registrationEnabled: false`, all social logins false) and advertises the
  `knowledgeable` endpoint.
- `GET /api/user`: Returns the local learner profile (`TUser`).
- `GET|POST /api/auth/refresh`: Mints a local session (`{ token, user }`) so the
  client's silent-refresh flow authenticates without credentials.
- `GET /api/endpoints`: Returns `{ "knowledgeable": { order, name } }` so the
  endpoint appears in the model selector.
- `GET /api/models`: Returns `{ "knowledgeable": [...] }` (`TModelsConfig`).

### 4.2 Conversations
- `GET /api/convos`: Returns `{ conversations: MinimalConversation[], nextCursor: null }`.
- `GET /api/convos/:id`: Returns one conversation (`TConversation`).
- `GET /api/convos/gen_title/:id`: Returns `{ title }`, lazily deriving and
  persisting a title from the first user message when none exists.
- `POST /api/convos/update`: Body `{ arg: { conversationId, title } }`; persists
  the title and returns the updated conversation.
- `DELETE /api/convos`: Body `{ arg: { conversationId } }`; cascades message
  deletion via the schema's foreign key.

### 4.3 Messages
- `GET /api/messages/:conversationId`: Returns chronologically ordered history as
  `TMessage[]`. The schema stores messages linearly; the adapter synthesizes the
  `parentMessageId` chain the client's message tree expects (`NO_PARENT` for the
  first message).

### 4.4 Chat & Streaming Turn
- `POST /api/agents/chat/:endpoint` (`:endpoint` = `knowledgeable`):
  - Request body: `TPayload` (camelCase) — `conversationId`, `parentMessageId`,
    `text`, `model`. `conversationId` values `new`, `PENDING`, empty, or absent
    create a fresh conversation; otherwise it must be a UUID.
  - Response: `text/event-stream`. Every frame is an unnamed SSE `message`
    (bare `data:` line) so the client's `message` listener receives it:
    1. `created` — the persisted user message. The client adopts its
       server-assigned `messageId` (and any new `conversationId`):
       `data: {"created": true, "message": { ...TMessage }}`
    2. Cumulative text deltas — `text` is the full reply so far, matching
       LibreChat's `createOnProgress` contract. `message: true` flags a message
       frame; `messageId` is the assistant message id (stable across the turn):
       `data: {"text": "...", "message": true, "initial": false, "conversationId": "...", "messageId": "...", "parentMessageId": "..."}`
    3. Terminal `final` — conversation plus request/response messages:
       `data: {"final": true, "conversation": {...}, "title": "...", "requestMessage": {...}, "responseMessage": {...}}`
  - On provider/stream failure the stream still terminates with a `final` event
    whose `responseMessage.error` is `true` and whose text is a stable,
    non-leaking message.

### 4.5 Knowledgeable Extensions
- `GET /api/concepts/:id/wiki`: Returns cached personal wiki page (`ConceptWikiPage`). _(M8, planned)_
- `GET /api/graph/neighborhood?concept_id=...&depth=...&limit=...`: Graph neighborhood for visualization. _(M9, implemented as an alias of canonical `GET /v1/graph/neighborhood`; same controller and error envelope. Client: `apps/web/client/src/knowledgeable/api/graphClient.ts` + `components/GraphExplorer.tsx`.)_

---

## 5. Concept-Aware Chat Rendering Architecture

### 5.1 Design Principles
1. **Unpolluted LLM Output**: The LLM emits standard Markdown. It is never instructed to output brittle HTML tags like `<span class="concept-known">` or custom XML markers that degrade reasoning quality and break LaTeX formulas.
2. **Deterministic Graph Grounding**: Concepts are matched against the learner's actual persistent knowledge graph.
3. **AST-Level Injection**: Annotations are resolved into the Unist Markdown AST, ensuring code blocks (`code`, `pre`), inline backticks, math delimiters (`$`, `$$`), and Markdown URLs are never accidentally highlighted.

### 5.2 The Processing Pipeline
```text
LLM Generation (Clean Markdown)
       │
       ▼
Tutor Turn Completion
  - Extract concepts mentioned in turn
  - Query SQLite for learner_confidence and concept metadata
  - Emit SSE event: `concept_annotations`
       │
       ▼
LibreChat Client Store
  - Stores message text and concept annotations mapping:
    `Map<messageId, ConceptAnnotation[]>`
       │
       ▼
Markdown Rendering Pipeline
  - `Markdown.tsx` -> `react-markdown`
  - Plugin `remarkConceptHighlight` visits narrative text nodes in AST
  - Replaces matched concept names with custom AST node `conceptHighlight`
  - `MarkdownComponents.tsx` renders `<ConceptHighlight>` component
       │
       ▼
Visual Output
  - Subtle underline or pill based on confidence:
    * Known (>= 0.80): Subtle dotted indicator + tooltip
    * Weak (< 0.80): Soft amber indicator + "Prerequisite needed" tooltip
    * New: Soft blue/violet emphasis + "New concept" tooltip
  - Hover: Displays concept title, learner confidence %, and brief definition
  - Click: Opens the Personal Knowledge Wiki drawer for that concept
```

---

## 6. Personal Knowledge Wiki Integration

### 6.1 Lifecycle & Maintenance Policy
- The wiki is a **persistent human-readable projection** of the learner graph.
- **Authoritative Graph Rule**: The graph remains authoritative. The wiki page is a derived, cached artifact.
- **Low-Frequency Maintenance**: Wiki pages are **not** dynamically regenerated on every visit.
  - Initial creation occurs when a concept reaches established status (`learner_confidence >= 0.70`).
  - Graph mutations flag the page as `is_stale = 1`.
  - Regeneration occurs asynchronously during idle sessions or on demand if stale, with rate-limiting.
- SQLite persistence table: `concept_wiki_pages` (see `docs/agent-context/data_models.md`).

### 6.2 UI Integration
- A dedicated slide-over panel component (`WikiDrawer.tsx`) within the LibreChat client layout.
- Can be triggered:
  - By clicking any highlighted concept in the chat conversation.
  - By clicking a concept node in the Graph Explorer.
  - By opening the "Personal Wiki" navigation sidebar item to browse all mastered concepts.
- Content renders using LibreChat's native Markdown/LaTeX/Code rendering components.

---

## 7. Upstream Maintenance & Upgrade Strategy

To prevent vendor lock-in and keep upstream merges straightforward:

1. **Repository Structure**:
   ```text
   Knowledgeable/
   ├── apps/
   │   └── web/                   # Vendored LibreChat client
   │       ├── src/
   │       │   ├── knowledgeable/ # ALL Knowledgeable-specific UI code
   │       │   │   ├── components/WikiDrawer.tsx
   │       │   │   ├── components/ConceptHighlight.tsx
   │       │   │   ├── components/GraphExplorer.tsx       # T9/M9 neighborhood panel (present)
   │       │   │   ├── graphTypes.ts / graphUtils.ts      # T9/M9 types + pure helpers (present)
   │       │   │   ├── plugins/remarkConcept.ts
   │       │   │   └── api/knowledgeableClient.ts
   │       │   │   └── api/graphClient.ts                 # T9/M9 /api/graph/neighborhood client (present)
   │       │   └── ... (LibreChat client source)
   ├── crates/                    # Rust backend crates
   ```
2. **Minimal Upstream Touchpoints**:
   Modifications to upstream files are strictly restricted to:
   - `client/src/components/Chat/Messages/Content/markdownConfig.ts` (registering `remarkConcept` and `ConceptHighlight`).
   - `client/src/components/Nav/Nav.tsx` (adding Knowledgeable Wiki and Graph navigation items).
   - `client/src/routes/ChatRoute.tsx` (mounting `WikiDrawer`).
   - `client/vite.config.ts` (configuring backend proxy port).
3. **Upstream Sync Procedure**:
   - Upstream LibreChat is tracked as a Git remote: `upstream https://github.com/danny-avila/LibreChat.git`.
   - Releases are merged using `git merge --no-commit upstream/main` or Git subtree updates, reviewing diffs against the isolated touchpoint files.

---

## 8. Provider & Authentication Architecture

1. **Separation of Concerns**:
   - **Learner Identity**: Governs access to the learner knowledge graph in SQLite. In single-user local mode, defaults to a local learner profile without credentials. In multi-user deployments, authenticated via standard JWT/session.
   - **Model Provider Authorization**: Governs LLM API access.
2. **Legitimate Provider Authorization (BYOK & Server Configuration)**:
   - **Bring Your Own Key (BYOK)**: Users can configure their own developer API keys in the client Settings modal (stored in secure browser storage or encrypted SQLite).
   - **Server Keys**: Deployment administrators can set `GEMINI_API_KEY`, `OPENAI_API_KEY`, or `ANTHROPIC_API_KEY` in `.env`.
   - **Local LLMs**: Out-of-the-box support for Ollama, vLLM, and llama.cpp via standard OpenAI-compatible base URLs (`http://localhost:11434/v1`).
3. **Consumer Subscription Clarification**:
   - Consumer ChatGPT Plus ($20/mo) and Gemini Advanced subscriptions **do not** provide public third-party API keys or OAuth grant flows for arbitrary chat applications.
   - Knowledgeable strictly uses official developer APIs and does not attempt fragile browser session hijacking.
