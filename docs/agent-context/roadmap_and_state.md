# Knowledgeable — Roadmap and State

## 1. State Rules

<roadmap_rule>
This file is the implementation state source for the coding agent.
Check items off only after the corresponding verification gate passes.
Do not mark work complete based on code presence alone.
</roadmap_rule>

Status vocabulary:

```text
[ ] not started
[-] in progress
[x] complete
[!] blocked
```

Current phase:

```text
Phase 0 — Project foundation (complete, 2026-09-09)
Next: Phase 1 — Conversational Tutor Skeleton
```

## 2. Phase 0 — Project Foundation

### Repository

- [x] Create Cargo workspace.
- [x] Create Flutter application under `apps/client`.
- [x] Create `/docs/agent-context` structure.
- [x] Configure formatting/linting for Rust and Flutter.
- [x] Configure CI baseline.

### Backend

- [x] Axum health endpoint.
- [x] PostgreSQL connection via SQLx.
- [x] Configuration loading.
- [x] Structured tracing.
- [x] Stable application error model.

### Client

- [x] Flutter app bootstrap.
- [x] Riverpod wiring.
- [x] go_router wiring.
- [x] Drift/SQLite initialization.
- [x] Typed API client foundation.

### Exit gate

```text
[x] cargo check passes (verified 2026-09-09, 2026-09-09 SQLite)
[x] cargo test passes (2 tests + doc-tests, domain decay)
[x] flutter analyze passes (No issues)
[x] flutter test passes (widget_test)
[x] backend health endpoint works with SQLite file auto-created (sqlite:knowledgeable.db, WAL/FKs, degraded if missing → ok after cargo run; no Docker/Postgres; verified via curl)
[x] Flutter client can reach backend in development (ApiClient baseUrl http://localhost:3000, health curl verified)
```

## 3. Phase 1 — Conversational Tutor Skeleton

### API

- [x] Conversation creation.
- [x] Message persistence.
- [x] Tutor streaming endpoint.
- [x] SSE event envelope.

### Client

- [x] Conversation list.
- [x] Chat screen.
- [x] Streaming text renderer.
- [x] Send/retry/error UX.

### Tutor

- [x] Provider-neutral `LlmClient`.
- [x] One configured model adapter.
- [x] Basic tutor prompt.
- [x] Conversation history handling.

### Exit gate

```text
[x] User can create conversation.
[x] User can send a learning question.
[x] Tutor streams a response.
[x] Conversation survives reload.
[x] No graph behavior is assumed by the client.
```

## 4. Phase 2 — Graph Foundation

### Database

- [ ] Implement `concept_nodes`.
- [ ] Implement `concept_relations`.
- [ ] Implement `learner_concept_states`.
- [ ] Implement `learners`.
- [ ] Add uniqueness/index constraints from `data_models.md`.

### Graph service

- [ ] `find_concept`.
- [ ] `get_concept`.
- [ ] `get_dependencies`.
- [ ] `get_related_concepts`.
- [ ] `get_learner_confidence`.
- [ ] `get_weak_dependencies`.
- [ ] Bounded recursive dependency traversal.

### Tutor

- [ ] Tool-calling loop.
- [ ] Typed graph tool contracts.
- [ ] Inject bounded `TutorContext`.
- [ ] Server-side learner authorization for every tool.

### Exit gate

```text
[ ] Tutor changes explanation when learner graph state changes.
[ ] Dependency direction is tested.
[ ] Weak prerequisites are discoverable.
[ ] Full-graph prompt dumps are impossible by design.
```

## 5. Phase 3 — Knowledge Construction

### Candidates

- [ ] `ConceptCandidate` persistence.
- [ ] `RelationCandidate` persistence.
- [ ] Tutor proposal tools.
- [ ] Candidate status lifecycle.
- [ ] Duplicate/identity resolution.

### Validation

- [ ] Schema validation.
- [ ] Canonical statement validation.
- [ ] World-confidence gate.
- [ ] Relation integrity checks.
- [ ] Atomic graph mutation transaction.
- [ ] Graph mutation audit record.

### Exit gate

```text
[ ] Tutor can propose missing concepts in an empty/sparse graph.
[ ] Candidates do not become authoritative automatically.
[ ] world_confidence < 0.80 is rejected.
[ ] Accepted concepts/relations commit atomically.
[ ] Client sees only confirmed mutations.
```

## 6. Phase 4 — Learner Health and Repair

### Confidence

- [ ] Learner confidence initialization.
- [ ] Bounded confidence updates.
- [ ] Simple long-term decay.
- [ ] `HEALTHY_THRESHOLD = 0.95` configuration.
- [ ] Weak concept query.
- [ ] Review item creation.

### Repair

- [ ] Detect confidence drops from teaching evidence.
- [ ] Identify weak dependency path.
- [ ] Tutor drills down to relevant foundation.
- [ ] Re-teach/repair.
- [ ] Re-estimate learner confidence.
- [ ] Resolve review item after sufficient evidence.

### Exit gate

```text
[ ] A weak concept can become review-eligible.
[ ] Tutor can repair a weak prerequisite without reviewing unrelated nodes.
[ ] Successful repair raises learner_confidence.
[ ] Long-term decay is covered by deterministic tests.
[ ] No composite mastery formula exists in v1.
```

## 7. Phase 5 — Personalized Frontier Teaching

- [ ] Distinguish healthy vs weak prerequisites in tutor context.
- [ ] Prefer strong known concepts as explanation anchors.
- [ ] Avoid unnecessary re-teaching of healthy concepts.
- [ ] Add tutor-driven iterative graph queries.
- [ ] Add empty/sparse graph progression tests.
- [ ] Add learner-level regression scenarios.

### Exit gate

```text
[ ] Same target produces materially different explanations for different learner graphs.
[ ] Tutor starts from the learner's frontier by default.
[ ] Tutor introduces only necessary missing concepts during a turn.
```

## 8. Phase 6 — Graph Inspection UX

### Client

- [ ] Concept neighborhood view.
- [ ] Semantic vs dependency visual distinction.
- [ ] Learner confidence visibility.
- [ ] Review/weak concept view.
- [ ] Post-session graph growth indicator.

### UX constraints

```text
[ ] Chat remains the primary workflow.
[ ] User is never required to manually maintain the graph.
[ ] Graph inspection is optional.
[ ] Mobile layout remains first-class.
```

## 9. Phase 7 — Sync, Offline Cache, and Reliability

- [ ] Drift cache models.
- [ ] Server-to-client graph synchronization.
- [ ] Conversation cache.
- [ ] Conflict policy: server wins for authoritative state.
- [ ] Retryable network errors.
- [ ] Offline read access for previously synchronized content.
- [ ] Cache invalidation/versioning.

### Exit gate

```text
[ ] App remains usable for cached conversations while offline.
[ ] Authoritative graph changes are never silently invented locally.
[ ] Reconnection converges local state to server state.
```

## 10. Phase 8 — Production Hardening

### Security

- [ ] Authentication.
- [ ] Authorization tests.
- [ ] Rate limiting.
- [ ] Secret management.
- [ ] Input/output size limits.
- [ ] Prompt-injection resilience tests.

### Reliability

- [ ] LLM timeout handling.
- [ ] Provider failure normalization.
- [ ] SQLite backup/restore verification (file copy + WAL checkpoint; no Postgres).
- [ ] Transaction retry strategy where safe (busy_timeout).
- [ ] SSE disconnect/reconnect behavior.

### Observability

- [ ] Request/turn IDs.
- [ ] Tool-call telemetry.
- [ ] Retrieval metrics.
- [ ] Validation rejection metrics.
- [ ] Learner-confidence change metrics.
- [ ] Token/cost telemetry.

### Exit gate

```text
[ ] Production deployment is repeatable.
[ ] Critical failure paths have tests.
[ ] Sensitive conversation content is not logged by default.
[ ] Backups have been restored successfully in a test environment.
```

## 11. Phase 9 — Retrieval Quality Experiments

Only start after the core loop works.

- [ ] Measure target resolution quality.
- [ ] Measure context size vs teaching quality.
- [ ] Measure dependency depth usefulness.
- [ ] Evaluate semantic retrieval needs.
- [ ] Add pgvector only if experiments justify it.
- [ ] Evaluate graph-centrality signals only if concrete failures justify them.

Do not add retrieval complexity before metrics show a problem.

## 12. Phase 10 — Advanced Learning System

Future; not part of v1:

- [ ] Richer confidence calibration.
- [ ] More nuanced learner evidence types.
- [ ] Advanced review scheduling.
- [ ] Conceptual topology analytics.
- [ ] Cross-domain concept synthesis.
- [ ] Better uncertainty/epistemic metadata if needed.
- [ ] Team/shared graphs only if product direction requires them.

## 13. Explicit Non-Goals Until Validated

```text
- Giant universal knowledge graph.
- Dedicated graph database.
- Complex SRS algorithm.
- Composite mastery score.
- Full graph embedded in prompts.
- Manual node-authoring workflow.
- Direct client graph mutation.
- Provider-specific tutor logic outside the LLM abstraction.
- Desktop-only interaction model.
```

## 14. Verification Gates

### Gate A — Domain

```text
[ ] data_models.md names match code.
[ ] invariants have unit tests.
[ ] confidence bounds enforced.
```

### Gate B — Graph

```text
[ ] dependency direction tested.
[ ] bounded traversal tested.
[ ] auth scoping tested.
[ ] duplicate relation rejection tested.
```

### Gate C — Tutor

```text
[ ] tool schemas tested.
[ ] sparse graph behavior tested.
[ ] learner-aware teaching tested.
[ ] weak-prerequisite repair tested.
```

### Gate D — Knowledge

```text
[ ] world-confidence threshold enforced.
[ ] candidate != authoritative node.
[ ] mutation transaction is atomic.
```

### Gate E — Client

```text
[ ] mobile chat works.
[ ] web chat works.
[ ] SSE stream renders correctly.
[ ] local cache survives restart.
```

## 15. Definition of Done for Any Checklist Item

<definition_of_done>
A roadmap item is complete only when implementation exists, relevant tests pass, formatting/lint/type checks pass, and the item does not violate `architecture.md`, `data_models.md`, or `tech_stack_and_rules.md`.
</definition_of_done>
