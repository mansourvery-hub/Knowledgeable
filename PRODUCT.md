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
- Transparency of tutor's reasoning (as required for pedagogical trust).

## Constraints
- Must function within a local-first or hybrid architecture (currently hybrid with Axum backend).
- Latency must be kept low through SSE streaming and intelligent state management.

## Non-goals
- Full curriculum generation (we focus on the learner's current frontier).
- Social features (collaboration).

## Important assumptions
- LLM tool-calling can correctly model pedagogical state.
- Knowledge graph is a reliable proxy for learner mastery.
