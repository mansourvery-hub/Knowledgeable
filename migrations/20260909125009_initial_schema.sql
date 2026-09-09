-- Knowledgeable initial schema — SQLite canonical (WAL, FKs, busy_timeout)
-- Per data_models.md canonical names, enums, and constraints.
-- Robust, modern: TEXT UUIDs, ISO8601 TEXT timestamps, REAL confidence, JSON TEXT, STRICT.
-- No PostgreSQL types, no pgcrypto, no pgvector. FKs enforced via PRAGMA foreign_keys=ON per connection (infrastructure/src/db.rs).
-- WAL and busy_timeout are set via SqliteConnectOptions, not via migration transaction.
-- Transactions guarantee atomic graph mutations.

-- ---------------------------------------------------------------------------
-- Learners
-- ---------------------------------------------------------------------------
CREATE TABLE learners (
    id TEXT PRIMARY KEY NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;

-- ---------------------------------------------------------------------------
-- Conversations / Messages
-- ---------------------------------------------------------------------------
CREATE TABLE conversations (
    id TEXT PRIMARY KEY NOT NULL,
    learner_id TEXT NOT NULL REFERENCES learners(id) ON DELETE CASCADE,
    title TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;
CREATE INDEX idx_conversations_learner_id ON conversations(learner_id);

CREATE TABLE conversation_messages (
    id TEXT PRIMARY KEY NOT NULL,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
    content TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;
CREATE INDEX idx_conversation_messages_conversation_created ON conversation_messages(conversation_id, created_at);

-- ---------------------------------------------------------------------------
-- Concept nodes — authoritative high-confidence statements
-- ---------------------------------------------------------------------------
CREATE TABLE concept_nodes (
    id TEXT PRIMARY KEY NOT NULL,
    canonical_name TEXT NOT NULL CHECK (length(canonical_name) > 0),
    canonical_statement TEXT NOT NULL CHECK (length(canonical_statement) > 0),
    learner_statement TEXT,
    world_confidence REAL NOT NULL CHECK (world_confidence >= 0 AND world_confidence <= 1),
    status TEXT NOT NULL CHECK (status IN ('active', 'archived')) DEFAULT 'active',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;
CREATE UNIQUE INDEX uq_concept_nodes_canonical_name ON concept_nodes(canonical_name);
CREATE INDEX idx_concept_nodes_status ON concept_nodes(status);

-- ---------------------------------------------------------------------------
-- Concept relations — two typed edges: semantic vs dependency
-- from_concept_id depends_on to_concept_id (canonical direction)
-- ---------------------------------------------------------------------------
CREATE TABLE concept_relations (
    id TEXT PRIMARY KEY NOT NULL,
    from_concept_id TEXT NOT NULL REFERENCES concept_nodes(id) ON DELETE CASCADE,
    to_concept_id TEXT NOT NULL REFERENCES concept_nodes(id) ON DELETE CASCADE,
    relation_type TEXT NOT NULL CHECK (relation_type IN ('semantic', 'dependency')),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    CHECK (from_concept_id <> to_concept_id),
    UNIQUE (from_concept_id, to_concept_id, relation_type)
) STRICT;
CREATE INDEX idx_concept_relations_to ON concept_relations(to_concept_id);
CREATE INDEX idx_concept_relations_type ON concept_relations(relation_type);

-- ---------------------------------------------------------------------------
-- Learner concept states — per-learner health
-- ---------------------------------------------------------------------------
CREATE TABLE learner_concept_states (
    learner_id TEXT NOT NULL REFERENCES learners(id) ON DELETE CASCADE,
    concept_id TEXT NOT NULL REFERENCES concept_nodes(id) ON DELETE CASCADE,
    learner_confidence REAL NOT NULL CHECK (learner_confidence >= 0 AND learner_confidence <= 1),
    last_evaluated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    last_reinforced_at TEXT,
    next_decay_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    PRIMARY KEY (learner_id, concept_id)
) STRICT;
CREATE INDEX idx_learner_states_learner_confidence ON learner_concept_states(learner_id, learner_confidence);

-- ---------------------------------------------------------------------------
-- Learner observations — evidence, not authoritative mutations
-- ---------------------------------------------------------------------------
CREATE TABLE learner_observations (
    id TEXT PRIMARY KEY NOT NULL,
    learner_id TEXT NOT NULL REFERENCES learners(id) ON DELETE CASCADE,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    concept_id TEXT REFERENCES concept_nodes(id) ON DELETE SET NULL,
    observation_type TEXT NOT NULL CHECK (observation_type IN ('understands','confusion','misconception','recall_failure','application_failure','new_understanding')),
    confidence_delta REAL CHECK (confidence_delta >= -1 AND confidence_delta <= 1),
    evidence TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;
CREATE INDEX idx_observations_learner ON learner_observations(learner_id, created_at);

-- ---------------------------------------------------------------------------
-- Candidate concept / relation — pending validation
-- ---------------------------------------------------------------------------
CREATE TABLE concept_candidates (
    id TEXT PRIMARY KEY NOT NULL,
    learner_id TEXT NOT NULL REFERENCES learners(id) ON DELETE CASCADE,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    canonical_name TEXT NOT NULL,
    canonical_statement TEXT NOT NULL,
    learner_statement TEXT,
    world_confidence REAL NOT NULL CHECK (world_confidence >= 0 AND world_confidence <= 1),
    status TEXT NOT NULL CHECK (status IN ('pending','accepted','rejected')) DEFAULT 'pending',
    rejection_reason TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    evaluated_at TEXT
) STRICT;
CREATE INDEX idx_concept_candidates_status_created ON concept_candidates(status, created_at);

CREATE TABLE relation_candidates (
    id TEXT PRIMARY KEY NOT NULL,
    learner_id TEXT NOT NULL REFERENCES learners(id) ON DELETE CASCADE,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    from_concept_id TEXT REFERENCES concept_nodes(id) ON DELETE SET NULL,
    from_candidate_id TEXT REFERENCES concept_candidates(id) ON DELETE SET NULL,
    to_concept_id TEXT REFERENCES concept_nodes(id) ON DELETE SET NULL,
    to_candidate_id TEXT REFERENCES concept_candidates(id) ON DELETE SET NULL,
    relation_type TEXT NOT NULL CHECK (relation_type IN ('semantic','dependency')),
    reason TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending','accepted','rejected')) DEFAULT 'pending',
    rejection_reason TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    evaluated_at TEXT,
    CHECK (
        (from_concept_id IS NOT NULL AND from_candidate_id IS NULL) OR
        (from_concept_id IS NULL AND from_candidate_id IS NOT NULL)
    ),
    CHECK (
        (to_concept_id IS NOT NULL AND to_candidate_id IS NULL) OR
        (to_concept_id IS NULL AND to_candidate_id IS NOT NULL)
    )
) STRICT;
CREATE INDEX idx_relation_candidates_status ON relation_candidates(status, created_at);

-- ---------------------------------------------------------------------------
-- Review items — health < HEALTHY_THRESHOLD = 0.95
-- ---------------------------------------------------------------------------
CREATE TABLE review_items (
    id TEXT PRIMARY KEY NOT NULL,
    learner_id TEXT NOT NULL REFERENCES learners(id) ON DELETE CASCADE,
    concept_id TEXT NOT NULL REFERENCES concept_nodes(id) ON DELETE CASCADE,
    status TEXT NOT NULL CHECK (status IN ('eligible','in_progress','resolved')) DEFAULT 'eligible',
    reason TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    resolved_at TEXT
) STRICT;
CREATE INDEX idx_review_items_learner_status ON review_items(learner_id, concept_id, status);

-- ---------------------------------------------------------------------------
-- Graph mutations — auditable atomic commits (payload as JSON TEXT)
-- ---------------------------------------------------------------------------
CREATE TABLE graph_mutations (
    id TEXT PRIMARY KEY NOT NULL,
    learner_id TEXT NOT NULL REFERENCES learners(id) ON DELETE CASCADE,
    conversation_id TEXT REFERENCES conversations(id) ON DELETE SET NULL,
    payload TEXT NOT NULL CHECK (json_valid(payload)),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;
CREATE INDEX idx_graph_mutations_learner ON graph_mutations(learner_id, created_at);

-- Note: updated_at is maintained by application (SET updated_at = now on UPDATE) for simplicity;
-- SQLite triggers would require recursive handling and are deferred until needed.
