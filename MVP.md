# MVP.md

## MVP Goal
Demonstrate an AI tutor that identifies and logs learner confusion/misunderstanding via reactive tool-calling during a standard chat session.

## Core User Journey
1. Learner asks a complex question.
2. Tutor checks the knowledge graph for weak prerequisites.
3. If weak prerequisites exist, tutor pivots to explain those first.
4. Learner's confusion is logged, and the knowledge graph updates confidence.

## Included Capabilities
- SSE Chat.
- `get_weak_dependencies` (tool).
- `propose_learner_update` (tool).
- `log_observation` (tool).
- Minimalist knowledge graph storage (SQLite).

## MVP Scope
The MVP stays centered on:
```text
chat
+
streaming
+
reactive tutor/tool use
+
learner graph/state
+
conversation persistence
```
Mermaid diagram rendering is explicitly retained as part of core chat rendering.

## Deliberately Out of MVP (Future, Not Rejected)
These are NOT MVP requirements, but they are NOT permanently rejected. Each is a
future capability with its own integration seam; see the canonical matrix in
`docs/agent-context/integration/librechat.md` and the staged roadmap in
`docs/agent-context/roadmap_and_state.md`:
- Conversation organization backend work still missing: bookmarks, pin, archive,
  fork/branch, conversation search, minimal sharing (product KEEP; backend gaps
  are post-MVP contracts, not scope cuts).
- Prompt slash commands, file attachments/uploads, file search/document RAG,
  web search, code execution, MCP (future tutor-tool capability; hide UI now,
  preserve the seam).
- Speech-to-text, text-to-speech, conversation/voice mode.
- Provider API keys / BYOK management UI, login/accounts/sync, token-usage UI,
  billing/credits (deployment-dependent futures).
- Reasoning/"thinking" display: optional user setting, OFF by default; never a
  dependency on private chain-of-thought.
- Disabled in MVP: technical parameter knobs, presets, generic traces, and
  artifacts as a product concept (retain shared rendering infra where cheap).

## Excluded Capabilities
- Cloud sync.
- Collaboration features.
- Advanced visualization of the knowledge graph.
- User authentication.

## Acceptance Criteria
- Chat correctly streams responses.
- Tutor identifies a prerequisite gap when a learner asks about a "frontier" topic.
- Knowledge graph persistence accurately reflects learner's current confidence.

## Known Limitations
- Knowledge graph is minimal (only learner state and basic dependencies).
- Single-user mode only (simulated).

## Deferred Features
- Full Graph Visualizer.
- Multi-user / Collaborative learning.
- Custom curriculum imports.
