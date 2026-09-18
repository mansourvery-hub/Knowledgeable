# TEST_STRATEGY.md

## Overview
We employ a multi-layered testing strategy to enforce the invariants defined in `QUALITY.md`.

## Layers of Verification

### 1. Unit Tests (Bricks)
- **Scope**: Individual functions, parsers, and small domain logic bricks.
- **Tools**: `cargo test` (Rust), standard Flutter `test`.
- **Focus**: Algorithmic correctness, boundary conditions.
- **Example**: Parsing an SSE chunk, verifying learner confidence bounds.

### 2. Integration Tests
- **Scope**: Interaction between layers (e.g., API to Service, Repository to DB).
- **Tools**: `sqlx::test` (in-memory SQLite).
- **Focus**: Database transactions, service orchestration, LLM provider shims.
- **Example**: `stream_tutor_turn` integration test (as currently implemented).

### 3. Verification of Invariants
- **API Key Validation**: Unit tests for `config` loading.
- **Streaming Integrity**: Integration tests simulating broken SSE chunks/network jitter.
- **CORS/Auth**: Integration tests validating headers and origins.

### 4. E2E (System) Validation
- **Scope**: Full browser-to-database flow.
- **Tools**: Playwright (via local system Chrome).
- **Focus**: User journeys, UI state consistency during streaming.

## Test Maintenance Rules
1. Every new bug must include a regression test in the relevant layer.
2. Every LLM tool added must have an accompanying test validating its output schema.
3. Every external service interaction must be mockable/verifiable.
