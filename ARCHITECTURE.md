# ARCHITECTURE.md

## Overview
The system follows a domain-driven, layer-based architecture designed for low-latency reactive teaching.

## Layer Structure
1. **API/Presentation Layer** (Axum): Handles HTTP/SSE requests and client connections.
2. **Application Layer** (Tutor Service): Orchestrates LLM interactions, tool dispatching, and state transitions.
3. **Domain Layer**: Contains core entities (Conversation, Learner, Concept, Relation) and logic.
4. **Infrastructure Layer**: Implements persistence (SQLite) and LLM client shims.

## State Management
- **Chat Controller**: Manages stream lifecycle and local UI state.
- **Tutor Logic**: Reactive, tool-driven state updates.

## Known Technical Debt
- **Repository Coupling**: `application` crate currently calls `infrastructure` repo implementations directly. 
  Future task: Introduce Repository Traits in `domain` and inject implementations.


## Known Technical Debt
- **Repository Coupling**:  crate currently calls  repo implementations directly. 
  Future task: Introduce Repository Traits in  and inject implementations.
