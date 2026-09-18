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
