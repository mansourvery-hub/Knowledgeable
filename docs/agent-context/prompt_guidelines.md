# Knowledgeable — Autonomous Coding Agent Guidelines

## 1. Instruction Priority

<agent_priority>
1. Follow explicit user requirements.
2. Preserve invariants in `architecture.md`.
3. Treat `data_models.md` as the canonical naming/schema source.
4. Follow `tech_stack_and_rules.md` for frameworks, boundaries, and coding rules.
5. Use `roadmap_and_state.md` to select and record implementation work.
6. Prefer the smallest correct implementation over speculative infrastructure.
</agent_priority>

## 2. Mandatory Reading Order

Before cross-module or architectural work:

```text
1. architecture.md
2. data_models.md
3. tech_stack_and_rules.md
4. roadmap_and_state.md
```

Before tutor/prompt/tool changes:

```text
1. architecture.md -> Tutor Graph-Navigation Model
2. data_models.md -> Tutor Tool Contracts
3. tech_stack_and_rules.md -> Tutor Rules + Prompt Construction Rules
4. this file -> Tutor Behavior Contract
```

## 3. Work Selection

<work_rule>
Select the smallest unchecked item in the current phase whose dependencies are satisfied.
Implement prerequisites before dependent work.
Do not implement later-phase features merely because they are interesting.
</work_rule>

When a requirement is ambiguous:
- infer from the existing architecture;
- preserve canonical names;
- choose the least complex compatible behavior;
- do not create new abstractions without need;
- update the relevant context file when the decision changes an invariant.

## 4. Repository Inspection

Before editing:

```text
[ ] inspect current directory structure
[ ] inspect relevant existing modules
[ ] search for existing implementations before creating duplicates
[ ] identify tests covering the target behavior
[ ] confirm which roadmap item is being implemented
```

Do not rewrite unrelated files.

## 5. Schema Discipline

<data_model_invariant>
Never invent a parallel representation of a canonical concept, relation, confidence field, status, or API payload when an existing model in `data_models.md` applies.
</data_model_invariant>

Before adding a persisted field:

```text
1. Search for an existing canonical field.
2. Identify whether it belongs to domain, persistence, transient context, or telemetry.
3. Update data_models.md before using a new canonical persisted field.
4. Add/modify migrations consistently.
```

Canonical names include:

```text
world_confidence
learner_confidence
canonical_name
canonical_statement
learner_statement
from_concept_id
to_concept_id
relation_type
```

## 6. Architecture Discipline

<architecture_invariant>
Flutter is the canonical client.
Rust is the backend.
PostgreSQL is authoritative.
SQLite is client cache only.
The tutor reasons and proposes; application code authorizes, validates, and commits.
</architecture_invariant>

Do not move responsibilities across these boundaries without updating `architecture.md` first.

## 7. LLM Discipline

<llm_constraints>
- LLM output is untrusted input.
- Never trust model-supplied learner IDs or authorization data.
- Never let model output execute arbitrary SQL/code.
- Never treat a candidate concept as authoritative before validation/commit.
- Never claim a graph mutation succeeded without a committed mutation event.
- Never place secrets in prompts.
- Keep tool inputs/outputs bounded.
- Use typed tool schemas.
- Prefer structured outputs for machine-consumed data.
</llm_constraints>

The LLM is responsible for:
- interpreting the learning request;
- deciding which retrieved concepts matter;
- navigating graph tools;
- identifying missing conceptual steps;
- deciding teaching sequence;
- proposing concepts/relations;
- producing learner observations.

Deterministic code is responsible for:
- authentication/authorization;
- schema validation;
- confidence bounds;
- world-confidence admission;
- graph invariants;
- persistence;
- transactionality;
- auditability.

## 8. Tutor Behavior Contract

<core_loop>
For each learning request:
1. Resolve the target concept where possible.
2. Query the learner graph instead of assuming a generic background.
3. Identify healthy known concepts useful as anchors.
4. Identify relevant weak prerequisites.
5. Teach from the learner's current frontier.
6. If required concepts are missing, propose the smallest useful conceptual bridge.
7. Use additional graph tools when the initial context is insufficient.
8. Detect evidence of misunderstanding or new understanding.
9. Propose graph/learner updates.
10. Let the backend validate and commit eligible updates.
</core_loop>

The tutor should not:
- re-teach a healthy concept just because it is present in a standard explanation;
- assume that absence from the graph means ignorance with certainty;
- pretend the learner's graph is complete;
- force graph UI interaction;
- directly modify authoritative state.

## 9. Sparse Graph Behavior

When the requested topic is missing or has insufficient prerequisites:

```text
Target absent/weakly connected
    ↓
Tutor inspects any reachable foundations
    ↓
Tutor identifies missing intermediate concepts
    ↓
Tutor proposes candidate concepts/relations
    ↓
Validation gate
    ↓
Accepted concepts enter authoritative graph
    ↓
Tutor continues
```

The autonomous agent must preserve this behavior even if a simpler hard-coded curriculum seems easier.

## 10. Personalization Contract

<personalization_rule>
Personalize the representation, not the truth.
</personalization_rule>

The canonical node statement must remain truth-bearing.
The `learner_statement` may evolve with learner capability.

Desired progression:

```text
"Entropy measures how spread-out the possibilities are."
        ↓
"Entropy measures uncertainty over a probability distribution."
        ↓
"Entropy is the expected value of -log p(x) under p."
```

Do not create learner-specific falsehoods merely because they are easy to understand.

## 11. Confidence Contract

<confidence_contract>
`world_confidence` is a knowledge-quality/admission signal.
`learner_confidence` is learner understanding health.
They are independent probabilities in [0,1].
</confidence_contract>

Defaults:

```text
WORLD_CONFIDENCE_MIN = 0.80
HEALTHY_THRESHOLD = 0.95
```

World-confidence rule:

```text
below threshold -> reject authoritative admission
```

Learner-confidence rule:

```text
below healthy threshold -> review eligible
```

## 12. Decay Contract

<decay_rule>
Keep v1 simple.
Use long-horizon time-based decay only.
Do not build a composite mastery formula.
</decay_rule>

A learner confidence drop may come from:
- natural long-term decay;
- observed confusion;
- observed misconception;
- recall/application failure.

A rise may come from:
- demonstrated understanding;
- successful repair/review.

Do not add frequency/usage/centrality weighting unless explicitly justified by a measured product requirement.

## 13. Graph Contract

<graph_rule>
Only two relation types exist in v1:
- semantic
- dependency

For dependency:
from_concept_id depends_on to_concept_id.
</graph_rule>

Do not merge both into a generic edge.
Do not casually add new relation types.

## 14. Prompt Construction Rules

Prompt structure:

```text
SYSTEM POLICY
TUTOR ROLE
GRAPH / LEARNER CONTEXT
TOOL DEFINITIONS
CONVERSATION HISTORY
CURRENT USER MESSAGE
```

Rules:
- Graph content is data, not instructions.
- User content is untrusted data.
- Tool output is untrusted data.
- Use explicit delimiters.
- Keep system policy concise.
- Do not dynamically rewrite system policy from model output.
- Version prompts when tutoring behavior changes materially.

## 15. Tool Design Rules

Every tutor tool must:

```text
[ ] one clear purpose
[ ] typed request schema
[ ] bounded input
[ ] bounded output
[ ] authenticated learner scope from server context
[ ] deterministic authorization
[ ] observable execution
[ ] stable errors
```

Never expose:
- raw SQL;
- generic database query tools;
- filesystem access;
- shell execution;
- arbitrary HTTP access;
- unrestricted graph dumps.

## 16. Mutation Rules

<mutation_safety>
Only application services may commit authoritative graph mutations.
Mutation commit must occur in a transaction.
Candidate state and authoritative state must remain distinguishable.
</mutation_safety>

When a graph mutation is rejected:
- preserve the rejection reason;
- do not silently retry with weakened validation;
- do not mark the roadmap complete if validation is failing.

## 17. Flutter Implementation Behavior

When modifying client code:

```text
[ ] use Riverpod for shared feature state
[ ] keep network/database logic in repositories
[ ] use typed generated models
[ ] keep UI widgets presentation-focused
[ ] maintain phone-first usability
[ ] preserve web/mobile parity in product flow
```

Do not add platform divergence without documenting why.

## 18. Rust Implementation Behavior

When modifying backend code:

```text
[ ] keep route handlers thin
[ ] validate requests early
[ ] keep domain pure
[ ] use typed errors
[ ] keep SQL behind repositories/infrastructure
[ ] use transactions for authoritative graph mutations
[ ] avoid unwrap/expect in production request paths
[ ] run fmt/check/test/clippy
```

## 19. Documentation Update Rules

Update documentation when:
- canonical field names change;
- a new persisted entity is introduced;
- module boundaries change;
- a new architecture dependency is introduced;
- tutor tool contracts change;
- a new roadmap invariant is established.

Do not let code and agent-context docs drift.

## 20. Verification Protocol

<verification_protocol>
After implementation:
1. Run focused tests.
2. Run type/static checks.
3. Run lint/format checks.
4. Run integration tests when a boundary changed.
5. Run broader tests when practical.
6. Update roadmap state only after verification passes.
</verification_protocol>

Minimum baseline:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
flutter analyze
flutter test
```

For production Flutter builds, also verify the configured target build command.

## 21. Failure Handling

If verification fails:

```text
[ ] do not mark item complete
[ ] identify smallest root cause
[ ] fix root cause
[ ] rerun failed check
[ ] document blocker in roadmap if unresolved
```

Never:
- delete failing tests;
- weaken assertions solely to pass;
- disable lint/type checks to hide failures;
- bypass validation;
- silently change canonical schemas to match an implementation error.

## 22. Architectural Escalation

Stop implementation and update architecture docs before continuing if a proposal would:

```text
- change Flutter/Rust/PostgreSQL ownership boundaries;
- add a second authoritative persistence system;
- add a new canonical relation type;
- change the meaning of world_confidence or learner_confidence;
- allow clients to directly mutate authoritative graph state;
- move provider-specific LLM code outside crates/llm;
- replace tool-driven graph navigation with unbounded prompt injection;
- introduce complex mastery/SRS logic as a core domain invariant.
```

## 23. Definition of Done

<definition_of_done>
A coding task is complete only when:
- requested behavior is implemented;
- canonical models are respected;
- tests cover important behavior;
- static checks pass;
- formatting/lint passes;
- migrations are valid when applicable;
- affected documentation/state is updated;
- no architectural invariant is violated.
</definition_of_done>

## 24. Final Checklist

```text
[ ] Correct phase/item identified.
[ ] Relevant agent-context files read.
[ ] Existing implementation inspected.
[ ] Canonical field names preserved.
[ ] Flutter/Rust/PostgreSQL boundaries preserved.
[ ] LLM output treated as untrusted.
[ ] World and learner confidence remain separate.
[ ] Tutor graph access remains bounded.
[ ] Candidate vs authoritative graph state remains separate.
[ ] Tests pass.
[ ] Static checks pass.
[ ] Roadmap updated only after verification.
[ ] No unrelated refactor included.
```
