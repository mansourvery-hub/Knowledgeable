# PRODUCT.md

## Problem
Learners often struggle to maintain momentum because they encounter "frontier" concepts—prerequisites they don't fully understand—without a structured way to identify and fill those gaps.

## Target users
Self-directed learners, students, and professionals acquiring complex new skills.

## Core user journeys
1. **Chat/Exploration**: Learner asks a question, tutor provides a scaffolded explanation, tutor identifies weak prerequisites and adds them to the graph.
2. **Knowledge Graph Expansion**: Tutor observes learner confusion and logs it, automatically updating the learner's confidence and potentially proposing new concepts or relationships.
3. **Reactive Teaching**: Tutor forces prerequisite checks before complex explanations.

## Functional requirements
- Conversational chat with LLM.
- Reactive tool calling (prerequisite checks, observation logging, graph updates).
- Persistence of conversation history and knowledge graph.

## UX requirements
- Minimalist, distraction-free chat.
- Real-time stream processing.
- Pedagogical transparency for trust: the tutor explains *why* it is teaching a prerequisite. Model "thinking"/reasoning display is an optional user setting, OFF by default, and must never depend on exposing private internal chain-of-thought.

## Constraints
- Must function within a local-first or hybrid architecture (currently hybrid with Axum backend).
- Latency must be kept low through SSE streaming and intelligent state management.

## Non-goals
- Full curriculum generation (we focus on the learner's current frontier).
- Full social/collaboration surfaces: followers, social profiles, comments, collaborative editing, public communities, content feeds, social discovery.
- Minimal "Share this conversation" is a deliberate, narrow exception to the above (post-MVP): a learner may share a useful explanation/conversation. It must not grow into collaboration or a content ecosystem.

## Product direction
- Knowledgeable is a focused AI learning environment built on LibreChat's mature chat UX and adapted to a Rust/SQLite pedagogical backend. LibreChat supplies commodity infrastructure; Knowledgeable supplies the learner model, knowledge graph, prerequisite awareness, reactive teaching, pedagogical state, concept highlighting, and the personal knowledge wiki.
- Cleanup goal: expose only the features that support Knowledgeable, keep useful future capabilities possible, and minimize divergence from upstream.
- Canonical feature policy (KEEP / DISABLE / REMOVE-from-surface / FUTURE, with product decision recorded separately from backend status) lives in `docs/agent-context/integration/librechat.md`. The staged cleanup roadmap lives in `docs/agent-context/roadmap_and_state.md`.

## Important assumptions
- LLM tool-calling can correctly model pedagogical state.
- Knowledge graph is a reliable proxy for learner mastery.
