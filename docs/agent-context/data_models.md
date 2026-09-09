# Knowledgeable — Data Models

## 1. Source of Truth

<data_model_rule>
These identifiers and field names are canonical. Do not invent synonymous persisted fields, API fields, or domain properties when an existing definition applies.
</data_model_rule>

Rules:
- IDs are opaque UUIDs.
- Backend timestamps are PostgreSQL `timestamptz` and serialized as ISO-8601 UTC strings.
- Probabilities are inclusive `[0, 1]`.
- Confidence values must never be represented as percentages in stored/API models.
- `world_confidence` and `learner_confidence` are separate concepts and must never be conflated.

## 2. Constants

```rust
pub const WORLD_CONFIDENCE_MIN: f32 = 0.80;
pub const HEALTHY_THRESHOLD: f32 = 0.95;
pub const DEFAULT_DECAY_GRACE_YEARS: f32 = 2.0;
pub const DEFAULT_DECAY_HALF_LIFE_YEARS: f32 = 12.0;
pub const DEFAULT_MAX_GRAPH_DEPTH: u8 = 3;
pub const DEFAULT_MAX_CONTEXT_CONCEPTS: usize = 64;
pub const DEFAULT_MAX_RELATED_CONCEPTS: usize = 16;
pub const DEFAULT_MAX_TOOL_RESULT_CONCEPTS: usize = 32;
```

Decay constants are configurable; they are not a claim about human cognition. See `architecture.md -> Learner Graph Repair` and `tech_stack_and_rules.md -> Learner Confidence Rules`.

## 3. Enumerations

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConceptStatus {
    Active,
    Archived,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelationType {
    Semantic,
    Dependency,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateStatus {
    Pending,
    Accepted,
    Rejected,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationType {
    Understands,
    Confusion,
    Misconception,
    RecallFailure,
    ApplicationFailure,
    NewUnderstanding,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewStatus {
    Eligible,
    InProgress,
    Resolved,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}
```

## 4. Concept Node

A `ConceptNode` represents a fundamentally true/high-confidence statement or definition suitable for the authoritative learner graph.

```rust
pub struct ConceptNode {
    pub id: Uuid,
    pub canonical_name: String,
    pub canonical_statement: String,
    pub learner_statement: Option<String>,
    pub world_confidence: f32,
    pub status: ConceptStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

Invariants:

```text
canonical_name is non-empty
canonical_statement is non-empty
0 <= world_confidence <= 1
world_confidence >= WORLD_CONFIDENCE_MIN for authoritative nodes
status == Active for normal tutoring/retrieval
```

### Meaning of fields

- `canonical_name`: stable identity/lookup label.
- `canonical_statement`: truth-bearing statement/definition; must remain learner-independent.
- `learner_statement`: current learner-specific formulation; may evolve as the learner gains conceptual tools.
- `world_confidence`: confidence that the canonical statement is sufficiently trustworthy to teach/store.

## 5. Learner Concept State

```rust
pub struct LearnerConceptState {
    pub learner_id: Uuid,
    pub concept_id: Uuid,
    pub learner_confidence: f32,
    pub last_evaluated_at: DateTime<Utc>,
    pub last_reinforced_at: Option<DateTime<Utc>>,
    pub next_decay_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

Invariants:

```text
0 <= learner_confidence <= 1
learner_id comes from authenticated server context
concept_id references a ConceptNode
```

Interpretation:

```text
learner_confidence = estimated current health of the learner's understanding of the concept.
```

Target:

```text
learner_confidence >= HEALTHY_THRESHOLD
```

Below threshold, the concept is review-eligible.

## 6. Graph Relation

```rust
pub struct ConceptRelation {
    pub id: Uuid,
    pub from_concept_id: Uuid,
    pub to_concept_id: Uuid,
    pub relation_type: RelationType,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

Semantics:

```text
relation_type == Semantic
    -> from_concept_id has a general/reference/association relationship to to_concept_id

relation_type == Dependency
    -> from_concept_id depends_on to_concept_id
```

Dependency direction is authoritative:

```text
A -> B [dependency]
means
A depends on B
```

Do not reverse this interpretation.

Do not add relation weights in v1.

## 7. Conversation

```rust
pub struct Conversation {
    pub id: Uuid,
    pub learner_id: Uuid,
    pub title: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

## 8. Conversation Message

```rust
pub struct ConversationMessage {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub role: MessageRole,
    pub content: String,
    pub created_at: DateTime<Utc>,
}
```

Tool calls/results and tutor events should use structured records; do not overload `content` with hidden machine state.

## 9. Tutor Event

Transient-to-stream event model:

```rust
pub enum TutorEvent {
    TextDelta { text: String },
    ToolCallStarted { tool_name: String, call_id: String },
    ToolCallFinished { tool_name: String, call_id: String },
    GraphUpdatePending,
    GraphUpdateCommitted { mutation_id: Uuid },
    ReviewSuggested { concept_ids: Vec<Uuid> },
    TurnCompleted { turn_id: Uuid },
    Error { code: String, message: String },
}
```

API serialization must be versioned and stable. See `tech_stack_and_rules.md -> SSE Rules`.

## 10. Learner Observation

Observations are evidence, not authoritative graph mutations.

```rust
pub struct LearnerObservation {
    pub id: Uuid,
    pub learner_id: Uuid,
    pub conversation_id: Uuid,
    pub concept_id: Option<Uuid>,
    pub observation_type: ObservationType,
    pub confidence_delta: Option<f32>,
    pub evidence: String,
    pub created_at: DateTime<Utc>,
}
```

Rules:
- `confidence_delta` must be within `[-1, 1]`.
- `evidence` is untrusted model-generated text.
- `concept_id` may be null for concepts that do not yet exist.
- Observations do not directly bypass learner-state policy.

## 11. Candidate Concept

```rust
pub struct ConceptCandidate {
    pub id: Uuid,
    pub learner_id: Uuid,
    pub conversation_id: Uuid,
    pub canonical_name: String,
    pub canonical_statement: String,
    pub learner_statement: Option<String>,
    pub world_confidence: f32,
    pub status: CandidateStatus,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub evaluated_at: Option<DateTime<Utc>>,
}
```

A candidate is never authoritative merely because an LLM proposed it.

## 12. Candidate Relation

Candidate references must be explicit.

```rust
pub enum ConceptRef {
    Existing { concept_id: Uuid },
    Candidate { candidate_id: Uuid },
}

pub struct RelationCandidate {
    pub id: Uuid,
    pub learner_id: Uuid,
    pub conversation_id: Uuid,
    pub from: ConceptRef,
    pub to: ConceptRef,
    pub relation_type: RelationType,
    pub reason: String,
    pub status: CandidateStatus,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub evaluated_at: Option<DateTime<Utc>>,
}
```

Resolution rule:
- Candidate references are resolved inside validation/commit.
- Persisted authoritative relations always contain authoritative `concept_id` values.

## 13. Review Item

```rust
pub struct ReviewItem {
    pub id: Uuid,
    pub learner_id: Uuid,
    pub concept_id: Uuid,
    pub status: ReviewStatus,
    pub reason: String,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}
```

Initial rule:

```text
learner_confidence < HEALTHY_THRESHOLD
    -> eligible for review
```

Do not implement complex SRS scheduling in v1.

## 14. Tutor Context

Transient typed context assembled for one tutor turn.

```rust
pub struct TutorContext {
    pub target_concept_id: Option<Uuid>,
    pub target_query: String,
    pub concepts: Vec<TutorContextConcept>,
    pub relations: Vec<TutorContextRelation>,
    pub weak_concept_ids: Vec<Uuid>,
}

pub struct TutorContextConcept {
    pub id: Uuid,
    pub canonical_name: String,
    pub canonical_statement: String,
    pub learner_statement: Option<String>,
    pub world_confidence: f32,
    pub learner_confidence: Option<f32>,
}

pub struct TutorContextRelation {
    pub from_concept_id: Uuid,
    pub to_concept_id: Uuid,
    pub relation_type: RelationType,
}
```

`TutorContext` is bounded by the constants in this file and may be expanded through tools.

## 15. Graph Mutation

A validated mutation is the application-level atomic unit for authoritative graph changes.

```rust
pub struct GraphMutation {
    pub concepts: Vec<ConceptCandidate>,
    pub relations: Vec<RelationCandidate>,
    pub learner_updates: Vec<LearnerUpdate>,
}

pub struct LearnerUpdate {
    pub concept_id: Uuid,
    pub new_learner_confidence: f32,
    pub reason: String,
}
```

Rules:

```text
new_learner_confidence must be within [0,1]
concept admission must satisfy WORLD_CONFIDENCE_MIN
all relation endpoints must resolve
mutation commits atomically
```

## 16. API Models

### Send message request

```json
{
  "content": "Why does quantum tunneling happen?"
}
```

### SSE event envelope

```json
{
  "version": 1,
  "event": "text_delta",
  "data": {
    "text": "..."
  }
}
```

Event names correspond to `TutorEvent` variants using stable snake_case serialization.

### Graph query response

```json
{
  "concepts": [],
  "relations": [],
  "weak_concept_ids": []
}
```

Do not expose database-specific rows directly through HTTP.

## 17. Tutor Tool Contracts

### `find_concept`

```json
{
  "query": "quantum tunneling",
  "limit": 8
}
```

Returns bounded candidate concept summaries.

### `get_concept`

```json
{
  "concept_id": "uuid"
}
```

Returns `TutorContextConcept`.

### `get_dependencies`

```json
{
  "concept_id": "uuid",
  "depth": 3
}
```

Returns bounded concepts + dependency relations.

### `get_related_concepts`

```json
{
  "concept_id": "uuid",
  "limit": 16
}
```

Returns bounded semantic neighbors.

### `get_learner_confidence`

```json
{
  "concept_ids": ["uuid"]
}
```

Returns learner confidence scoped to the authenticated learner.

### `get_weak_dependencies`

```json
{
  "concept_id": "uuid",
  "threshold": 0.95,
  "depth": 3
}
```

Returns required concepts below the requested threshold.

### `propose_concept`

```json
{
  "canonical_name": "Potential barrier",
  "canonical_statement": "A region where potential energy is higher than the particle's available energy.",
  "learner_statement": "A region that is too energetically high for the particle to cross classically.",
  "world_confidence": 0.97
}
```

Returns a pending candidate identifier. It does not commit an authoritative node.

### `propose_relation`

```json
{
  "from": { "candidate_id": "uuid" },
  "to": { "existing_concept_id": "uuid" },
  "relation_type": "dependency",
  "reason": "The new concept requires the existing concept."
}
```

The exact serialized `ConceptRef` representation must be documented in the implementation and tested. Never accept arbitrary model-generated IDs as authoritative.

### `propose_learner_update`

```json
{
  "concept_id": "uuid",
  "new_learner_confidence": 0.91,
  "reason": "Learner failed to explain the prerequisite without prompting."
}
```

Server validates the proposed value and evidence before applying it.

## 18. Database Tables

Authoritative table set for v1:

```text
learners
conversations
conversation_messages
concept_nodes
concept_relations
learner_concept_states
learner_observations
concept_candidates
relation_candidates
review_items
graph_mutations
```

Use UUID primary keys where applicable. Enforce uniqueness/indexes for concept identity and relation duplication.

Recommended constraints/indexes:

```text
concept_nodes(canonical_name)
concept_relations(from_concept_id, to_concept_id, relation_type) UNIQUE
learner_concept_states(learner_id, concept_id) UNIQUE
review_items(learner_id, concept_id, status)
conversation_messages(conversation_id, created_at)
concept_candidates(status, created_at)
```

## 19. Learner Confidence Update Contract

<confidence_contract>
`learner_confidence` is the health estimate for one learner/concept pair.
It changes because of explicit evidence or conservative long-term decay.
Do not introduce a composite mastery formula in v1.
</confidence_contract>

Allowed sources:
- initial creation
- successful demonstrated understanding
- detected confusion/misconception
- recall/application failure
- explicit repair outcome
- natural time-based decay

All updates are bounded to `[0,1]` and recorded with a reason.

## 20. Personalization Contract

<personalization_rule>
Personalize the representation, not the truth.
</personalization_rule>

Example progression for one concept:

```text
Beginner:
"Entropy measures how spread-out the possibilities are."

Intermediate:
"Entropy measures uncertainty over a probability distribution."

Advanced:
"Entropy is the expected value of -log p(x) under p."
```

The canonical concept remains truth-bearing across all stages. The learner-specific `learner_statement` can evolve.
