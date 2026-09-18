# QUALITY.md

## Architectural Invariants
- **Layer Separation**: Presentation layer (API) must not access persistence (Repositories) directly.
- **Dependency Flow**: Application layer orchestrates domain entities, not vice-versa.
- **CORS Policy**: CORS must be explicitly locked to allowed origins (frontend-only) in production.

## Reliability Requirements
- **Streaming Robustness**: Network interruptions during SSE streaming must not cause deadlocks (must trigger client-side cleanup).
- **Graceful Degradation**: If LLM provider is unreachable, API must return descriptive 503/ServiceUnavailable.
- **State Integrity**: Learner confidence updates in the graph must be atomic (transactional SQLite).

## Security Requirements
- **Credential Handling**: LLM API keys must be loaded via environment and never written to logs or telemetry.
- **Input Sanitization**: All user-provided chat content must be sanitized before reaching the LLM/graph prompt.

## Performance Requirements
- **Initial Latency**: First byte of an LLM response must be delivered within 1s.
- **Throughput**: System must support concurrently streaming multiple conversations without thread starvation (Tokio context).

## Verification Requirements
- **Key Presence**: Every request attempting LLM interaction must have a validated API key.
- **Stream Integrity**: LLM responses must be parsed as valid SSE events; partial/broken chunks must trigger an error event rather than a crash.
