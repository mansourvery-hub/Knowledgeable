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
  - (Planned, not built: a "Personal Wiki" navigation entry to browse all
    mastered concepts — see roadmap feedback F8.)
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

---

## 9. Authoritative Feature-Policy Matrix (Canonical)

This section is the canonical product direction for the bundled LibreChat
frontend. It supersedes any earlier, more aggressive audit notes.

**Status vocabulary.** `KEEP / DISABLE / REMOVE / FUTURE` describes **product
direction, not necessarily immediate code deletion**:
- `KEEP` — intended product surface. It may still need backend work; that gap
  is a future implementation contract, not a reason to cut the feature.
- `DISABLE` — hide from the current product surface via configuration or a
  central UI gate. Implementation stays close to upstream.
- `REMOVE (from product surface)` — not part of the product. First hide/disable;
  delete implementation only later, after dependency analysis proves isolation.
- `FUTURE` — not MVP, deliberately preserved as a possible capability with a
  clean seam. Do not architect the cleanup to make it needlessly hard.

**Two columns are recorded separately** for every row: `product decision` and
`current backend/implementation status`. A product `KEEP` with a missing backend
route means "build it later", never "cut it now".

### 9.1 Core features — KEEP (not cleanup targets)

| Feature | Product | Backend status today |
| :--- | :--- | :--- |
| Normal chat | KEEP | Served: `POST /api/agents/chat/:endpoint` (legacy SSE + v2 ticket/stream/status) |
| SSE / real-time streaming | KEEP | Served (`created` / cumulative `text` deltas / `final`; `tool_progress` + `concept_annotations` frames) |
| Conversation history, open conversations | KEEP | Served (`GET /api/convos`, `GET /api/convos/:id`, `GET /api/messages/:conversationId`) |
| Conversation titles | KEEP | Served (`gen_title`, `POST /api/convos/update`, provisional title on turn start) |
| Delete conversation | KEEP | Served (`DELETE /api/convos` cascade) |
| Regenerate answers, edit previous messages, stop generation | KEEP | Client turn resend/abort against the chat endpoint; `GET .../status/:conversationId` authorizes teardown |
| Model selection, endpoint/model infrastructure | KEEP | Served (`GET /api/endpoints`, `GET /api/models`, per-turn provider dispatch incl. turn-scoped BYOK `apiKey`) |
| Markdown, LaTeX/math, code highlighting, copy, chat-input QoL, basic chat UX/a11y, theme/appearance, font size, RTL/LTR, auto-scroll, keep-screen-awake, smooth streaming | KEEP | Client-rendered; no dedicated backend route needed |
| Concept highlighting | KEEP | Served (`concept_annotations` frame + `remarkConceptHighlight` + `<ConceptHighlight>`) |
| Knowledge Graph explorer | KEEP | Served (`GET /api/graph/neighborhood` alias of canonical `GET /v1/graph/neighborhood`; `GraphExplorer` + search `GET /api/concepts/search`) |
| Personal Knowledge Wiki | KEEP | Served (`GET /api/concepts/:id/wiki` cached pages + `WikiDrawer`) |
| Concept search (graph pickers) | KEEP | Served (`GET /api/concepts/search`) |
| Local conversation export/import | KEEP | Client-side data controls; keep |
| Mermaid diagrams | KEEP | Explicitly retained rendering capability |

### 9.2 Conversation organization — KEEP (earlier audit was too aggressive)

| Feature | Product | Backend status today |
| :--- | :--- | :--- |
| Bookmarks (mark important learning conversations) | KEEP | **Missing**: no `/api/tags*` route; `interface.bookmarks: false` but `role()` still grants `BOOKMARKS.*` so the side-panel gate can still show it — future contract |
| Pin conversation | KEEP | **Missing**: no `/api/convos/pin`; `conv_json` hardcodes `pinned: false` — future contract |
| Archive conversation | KEEP | **Missing**: no `/api/convos/archive*`; `conv_json` hardcodes `isArchived: false` — future contract |
| Fork / branch conversations (explore explanation path A vs B without destroying the original) | KEEP | **Missing**: no `/api/convos/fork`, no `/api/messages/branch` — future contract |
| Search all conversations | KEEP (supporting capability for persistent learning history, not "generic clutter") | **Missing**: no `/api/search*` — future contract |
| Share conversation (narrow exception: "Share this conversation"; NO followers, profiles, comments, collaborative editing, communities, feeds, discovery) | KEEP (post-MVP) | **Missing**: no `/api/share*`; `sharedLinksEnabled: false` — future contract |

### 9.3 Removed from the current product surface (hide first; delete only if isolated)

Product `REMOVE` here means **remove from the visible product**, not "delete
every related source file on day one". These remain vendored in
`apps/web/client/src/routes/index.tsx`, `SidePanel/*`, settings `registry.tsx`,
and `packages/data-provider/src/api-endpoints.ts`, while the Rust adapter serves
none of their backends (all 404/undeclared):

Agent marketplace, agent builder, assistant builder, custom agents, skills,
projects, prompt library/editor/variables, scheduled chats, generic LibreChat
Memories (see §9.6), plugin marketplace, generic enterprise/admin UI, Langfuse
UI/integration, generic agent API-key management, 2FA UI, SaaS
registration/password-reset/social-login UI, and other account-management
surfaces with no Knowledgeable counterpart.

### 9.4 Disabled now, preserved for the future

| Feature | Product | Note |
| :--- | :--- | :--- |
| Prompt slash commands | FUTURE (disabled now) | Reusable prompts could serve tutoring later; do not turn the MVP into a prompt-management app |
| File uploads / attachments (PDFs, notes, screenshots, books, exercises) | FUTURE (disabled now) | Lack of `/api/files/*` is implementation state, not permanent rejection; keep the seam addable |
| File search / document RAG | FUTURE | Valuable later for learner-provided material |
| Web search | FUTURE / OPTIONAL | Never a hidden dependency of the core tutor |
| Code execution (displaying code stays KEEP) | FUTURE / OPTIONAL | Do not implement now |
| MCP | FUTURE / IMPORTANT POSSIBILITY (disabled UI now) | The future tutor may need MCP-compatible tools (references, docs, domain DBs, calculators). Hide UI, keep a clean integration boundary, keep pedagogical logic authoritative — never a generic MCP agent shell |
| Speech-to-text | FUTURE | Language learning, accessibility, hands-free, oral questioning |
| Text-to-speech | FUTURE | Pronunciation/listening, accessibility, spoken explanations |
| Conversation / voice mode | FUTURE | Pedagogically interesting; not MVP; no voice-product effort now |
| Provider API keys (BYOK) | FUTURE product capability (omit in MVP UI) | Backend already honors turn-scoped `apiKey`; future settings UI needs strong security boundaries, not the whole generic provider-management product |
| Login / accounts | FUTURE, BUT IMPORTANT | Needed for sync, cloud data, hosted deployments; single-user stub today (`GET /api/user`, `POST|GET /api/auth/refresh`) must not imply sync can never exist |
| Token usage UI | FUTURE / IMPORTANT FOR BYOK | Cost visibility matters with user keys; keep out of MVP main UI |
| Billing / credits | FUTURE / DEPLOYMENT-DEPENDENT | Relevant only for hosted SaaS; never architect the MVP around billing |

### 9.5 Reasoning display ("show thinking")

Policy: **available as an optional user setting, OFF by default**
(`showThinkingAtom` / `registry.tsx` `showThinking`). Rationale: transparency
builds pedagogical trust, but always-on thinking is noisy and distracts from
learning. Distinguish pedagogical explanation ("why the tutor is teaching this
prerequisite") and supported model-generated reasoning summaries/status from
private/internal chain-of-thought — Knowledgeable never depends on exposing the
latter.

### 9.6 Generic LibreChat Memories — REMOVE (architectural decision)

Do not maintain two competing memory concepts. LibreChat-style preference memory
("prefers concise explanations") must not compete with the Knowledgeable
learner model ("low confidence in concept X, missing prerequisite Y"). The
graph/learner state stays authoritative; hide/remove Memories from the product
surface (`MemoryPanel`, `MemoryToggle`, `/api/memories*` stays unimplemented).

### 9.7 Parameters / presets / traces / artifacts

- **Parameters** — DISABLE: do not expose technical model knobs not backed by
  the Knowledgeable endpoint (today `interface.parameters: true`; cleanup sets
  it `false` via config).
- **Presets** — DISABLE, not MVP (`interface.presets: false`, no `/api/presets`).
- **Generic traces** — DISABLE: keep pedagogically useful tutor/tool transparency
  (`tool_progress` → `ToolActivity`) where required; do not expose generic infra
  telemetry (`Trace/*`, no `/api/traces*`).
- **Artifacts** — DISABLE/REVIEW: not a central product concept; retain shared
  rendering infra only where cheap.

---

## 10. Current Backend Boundary (What the Adapter Actually Serves)

Implemented in `crates/api/src/routes/librechat/` (`mod.rs`, `system.rs`,
`convos.rs`, `chat.rs`, plus `wiki.rs`, `concepts.rs`):

```text
GET  /api/config
GET  /api/user
GET|POST /api/auth/refresh            (local single-user session stub)
GET  /api/endpoints
GET  /api/models                      (env-truthful + local-tutor fallback)
GET  /api/roles/:role_name            (USER grants; unknown names 404)
GET  /api/convos                      (list; nextCursor null)
GET  /api/convos/:id
GET  /api/convos/gen_title/:id
POST /api/convos/update               ({ arg: { conversationId, title } })
DELETE /api/convos                    ({ arg: { conversationId } })
GET  /api/messages/:conversationId
POST /api/agents/chat/:endpoint       (legacy SSE or v2 start ticket)
GET  /api/agents/chat/stream/:stream_id
GET  /api/agents/chat/status/:conversationId
GET  /api/concepts/:id/wiki
GET  /api/concepts/search
GET  /api/concepts/mastered          (Phase 5a W1: known concepts ≥0.80, weakest-first + wiki_status + truncated)
GET  /api/graph/neighborhood          (alias of canonical /v1/graph/neighborhood)
```

Explicitly NOT implemented (any UI calling these degrades — the reason cleanup
hides the UI first): `/api/search*`, `/api/share*`, `/api/files*`,
`/api/tags*`, `/api/convos/{fork,pin,archive,duplicate}`, `/api/messages/branch`,
`/api/presets*`, `/api/prompts*`, `/api/skills*`, `/api/memories*`,
`/api/projects*`, `/api/schedules*`, `/api/mcp*`, `/api/agents*` (agent CRUD),
`/api/assistants*`, `/api/traces*`, `/api/balance`, 2FA/auth-provider/admin/
Langfuse surfaces. Known mismatch to handle in cleanup: `system.rs`
`interface.* = false` does NOT hide everything, because `system.rs` `role()`
grants every `PermissionTypes.*` capability and several side-panel entries
(`useSideNavLinks.ts`: prompts, bookmarks, memories, skills, MCP, files) gate
only on permissions, not on `interfaceConfig`.

---

## 11. Protected Boundaries

- `apps/web/client/src/knowledgeable/` — protected Knowledgeable boundary
  (concept highlighting, wiki drawer, graph explorer, API clients, stores).
  LibreChat cleanup must not modify it unless an explicit integration contract
  requires it.
- `crates/api/src/routes/librechat/` — narrow adapter contract. Add a backend
  route only for an actual Knowledgeable product requirement, never merely
  because a dormant frontend component expects it.
- Shared frontend infrastructure (chat rendering, message tree, React Query,
  endpoint selection, SSE, mobile layout, a11y, markdown, error handling) —
  protect even when it lives near a hidden feature.
- Intentionally upstream-shaped code — keep file structure, avoid renames,
  restructuring, abstraction rewrites, formatting churn, or unrelated import
  reorganization so a future upstream merge diffs cleanly.

---

## 12. Upstream-Maintenance Constraints (Mandatory)

1. **Prefer existing configuration switches.** If LibreChat exposes
   `webSearch / runCode / fileSearch / multiConvo / temporaryChat / presets /
   prompts / bookmarks / ... : false`, use that seam first; do not rewrite
   feature internals to hide them.
2. **Prefer existing capability/permission gates.** Reuse interface config,
   feature flags, capabilities, and `role()` permissions; do not invent a
   parallel flag framework without genuine necessity.
3. **Prefer centralized UI gates over feature-internal modification.** Change one
   navigation registry, menu registry, route registration, or capability mapping
   rather than dozens of files inside the feature. The ideal patch is small and
   obvious.
4. **Do not fork feature internals unnecessarily.** Central gate → UI
   unavailable, while the upstream implementation stays close to upstream. Never
   start by rewriting a feature's components/providers/routing.
5. **Keep Knowledgeable-specific code isolated** under
   `apps/web/client/src/knowledgeable/`.
6. **Preserve upstream file structure** — no gratuitous renames, moves, or churn.
7. **Keep the adapter contract narrow** — no backend route without a real
   Knowledgeable product requirement (prevents unused-UI → backend-work →
   maintenance-burden spirals).
8. **Do not confuse "product KEEP" with "implement immediately."**
   `Fork = KEEP` means the backend gap belongs in the future roadmap, not that
   the next PR builds it.
9. **Remove product surface before deleting infrastructure**: hide/disable →
   verify core flows → inspect dependencies → identify genuinely dead code →
   only then consider deleting isolated dead implementation. Never begin with
   `rm -rf feature-directory`.
10. **Protect shared infrastructure** supporting chat, rendering, state, SSE, and
    a11y.
11. **Every cleanup batch must be independently verifiable**: narrow scope,
    explicit expected behavior, verification contract, rollback point, small diff.
12. **Minimize divergence**: a 3-line configuration change beats a 300-line
    custom rewrite; isolate genuinely necessary custom code at an explicit
    Knowledgeable seam.
13. **Upstream updates are first-class**: the end state must support
    `pull upstream → resolve a small set of deliberate patches → run focused
    verification → continue`, with Knowledgeable changes easy to identify and
    gating changes centralized.

**Per-batch discipline:** `READ → MAP → CHANGE ONE SMALL SEAM → VERIFY →
INSPECT DIFF → COMMIT/BASELINE → NEXT SEAM`. Never hand an agent a giant list
and hope a rewrite still works; the project optimizes for controlled evolution,
not maximum deletion.

## 13. Product Philosophy

Knowledgeable is a focused AI learning environment built on LibreChat's mature
chat UX and adapted to a Rust/SQLite pedagogical backend. LibreChat supplies
commodity infrastructure; Knowledgeable supplies the learner model, knowledge
graph, prerequisite awareness, reactive teaching, pedagogical state, concept
highlighting, and the personal knowledge wiki. The target is "expose only the
features that support Knowledgeable, keep useful future capabilities possible,
and minimize divergence from upstream" — not "remove everything LibreChat
does". The staged execution contracts live in
`docs/agent-context/roadmap_and_state.md` §4.
