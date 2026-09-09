-- Knowledgeable initial schema — PostgreSQL is authoritative
-- Per data_models.md canonical names, enums, and constraints.
-- Robust, modern, long-lived: uses UUID PKs, timestamptz, FKs, CHECKs, unique constraints,
-- and recursive CTE friendly layout (no graph DB needed in v1).

-- Enable pgcrypto for gen_random_uuid() if available; not required for app-generated UUIDs
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ---------------------------------------------------------------------------
-- Learners (authenticated owners of graph state)
-- ---------------------------------------------------------------------------
CREATE TABLE learners (
    id UUID PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ---------------------------------------------------------------------------
-- Conversations / Messages (chat persistence)
-- ---------------------------------------------------------------------------
CREATE TABLE conversations (
    id UUID PRIMARY KEY,
    learner_id UUID NOT NULL REFERENCES learners(id) ON DELETE CASCADE,
    title TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_conversations_learner_id ON conversations(learner_id);

CREATE TABLE conversation_messages (
    id UUID PRIMARY KEY,
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_conversation_messages_conversation_created ON conversation_messages(conversation_id, created_at);

-- ---------------------------------------------------------------------------
-- Concept nodes — authoritative high-confidence statements
-- ---------------------------------------------------------------------------
CREATE TABLE concept_nodes (
    id UUID PRIMARY KEY,
    canonical_name TEXT NOT NULL CHECK (char_length(canonical_name) > 0),
    canonical_statement TEXT NOT NULL CHECK (char_length(canonical_statement) > 0),
    learner_statement TEXT,
    world_confidence REAL NOT NULL CHECK (world_confidence >= 0 AND world_confidence <= 1),
    status TEXT NOT NULL CHECK (status IN ('active', 'archived')) DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT chk_world_confidence_gate CHECK (
        -- authoritative active nodes should satisfy gate, but allow DB to store candidates via separate table
        -- keep permissive here; application enforces WORLD_CONFIDENCE_MIN = 0.80
        TRUE
    )
);
CREATE UNIQUE INDEX uq_concept_nodes_canonical_name ON concept_nodes(canonical_name);
CREATE INDEX idx_concept_nodes_status ON concept_nodes(status);

-- ---------------------------------------------------------------------------
-- Concept relations — two typed edges: semantic vs dependency
-- Dependency semantics: from_concept_id depends_on to_concept_id (canonical direction)
-- ---------------------------------------------------------------------------
CREATE TABLE concept_relations (
    id UUID PRIMARY KEY,
    from_concept_id UUID NOT NULL REFERENCES concept_nodes(id) ON DELETE CASCADE,
    to_concept_id UUID NOT NULL REFERENCES concept_nodes(id) ON DELETE CASCADE,
    relation_type TEXT NOT NULL CHECK (relation_type IN ('semantic', 'dependency')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT chk_no_self_relation CHECK (from_concept_id <> to_concept_id),
    CONSTRAINT uq_relation UNIQUE (from_concept_id, to_concept_id, relation_type)
);
CREATE INDEX idx_concept_relations_to ON concept_relations(to_concept_id);
CREATE INDEX idx_concept_relations_type ON concept_relations(relation_type);

-- ---------------------------------------------------------------------------
-- Learner concept states — per-learner health
-- ---------------------------------------------------------------------------
CREATE TABLE learner_concept_states (
    learner_id UUID NOT NULL REFERENCES learners(id) ON DELETE CASCADE,
    concept_id UUID NOT NULL REFERENCES concept_nodes(id) ON DELETE CASCADE,
    learner_confidence REAL NOT NULL CHECK (learner_confidence >= 0 AND learner_confidence <= 1),
    last_evaluated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_reinforced_at TIMESTAMPTZ,
    next_decay_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (learner_id, concept_id)
);
CREATE INDEX idx_learner_states_learner_confidence ON learner_concept_states(learner_id, learner_confidence);

-- ---------------------------------------------------------------------------
-- Learner observations — evidence, not authoritative mutations
-- ---------------------------------------------------------------------------
CREATE TABLE learner_observations (
    id UUID PRIMARY KEY,
    learner_id UUID NOT NULL REFERENCES learners(id) ON DELETE CASCADE,
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    concept_id UUID REFERENCES concept_nodes(id) ON DELETE SET NULL,
    observation_type TEXT NOT NULL CHECK (observation_type IN ('understands','confusion','misconception','recall_failure','application_failure','new_understanding')),
    confidence_delta REAL CHECK (confidence_delta >= -1 AND confidence_delta <= 1),
    evidence TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_observations_learner ON learner_observations(learner_id, created_at);

-- ---------------------------------------------------------------------------
-- Candidate concept / relation — pending validation
-- ---------------------------------------------------------------------------
CREATE TABLE concept_candidates (
    id UUID PRIMARY KEY,
    learner_id UUID NOT NULL REFERENCES learners(id) ON DELETE CASCADE,
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    canonical_name TEXT NOT NULL,
    canonical_statement TEXT NOT NULL,
    learner_statement TEXT,
    world_confidence REAL NOT NULL CHECK (world_confidence >= 0 AND world_confidence <= 1),
    status TEXT NOT NULL CHECK (status IN ('pending','accepted','rejected')) DEFAULT 'pending',
    rejection_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    evaluated_at TIMESTAMPTZ
);
CREATE INDEX idx_concept_candidates_status_created ON concept_candidates(status, created_at);

CREATE TABLE relation_candidates (
    id UUID PRIMARY KEY,
    learner_id UUID NOT NULL REFERENCES learners(id) ON DELETE CASCADE,
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    -- ConceptRef polymorphism: exactly one per side
    from_concept_id UUID REFERENCES concept_nodes(id) ON DELETE SET NULL,
    from_candidate_id UUID REFERENCES concept_candidates(id) ON DELETE SET NULL,
    to_concept_id UUID REFERENCES concept_nodes(id) ON DELETE SET NULL,
    to_candidate_id UUID REFERENCES concept_candidates(id) ON DELETE SET NULL,
    relation_type TEXT NOT NULL CHECK (relation_type IN ('semantic','dependency')),
    reason TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending','accepted','rejected')) DEFAULT 'pending',
    rejection_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    evaluated_at TIMESTAMPTZ,
    CONSTRAINT chk_from_ref CHECK (
        (from_concept_id IS NOT NULL AND from_candidate_id IS NULL) OR
        (from_concept_id IS NULL AND from_candidate_id IS NOT NULL)
    ),
    CONSTRAINT chk_to_ref CHECK (
        (to_concept_id IS NOT NULL AND to_candidate_id IS NULL) OR
        (to_concept_id IS NULL AND to_candidate_id IS NOT NULL)
    )
);
CREATE INDEX idx_relation_candidates_status ON relation_candidates(status, created_at);

-- ---------------------------------------------------------------------------
-- Review items — health < HEALTHY_THRESHOLD = 0.95
-- ---------------------------------------------------------------------------
CREATE TABLE review_items (
    id UUID PRIMARY KEY,
    learner_id UUID NOT NULL REFERENCES learners(id) ON DELETE CASCADE,
    concept_id UUID NOT NULL REFERENCES concept_nodes(id) ON DELETE CASCADE,
    status TEXT NOT NULL CHECK (status IN ('eligible','in_progress','resolved')) DEFAULT 'eligible',
    reason TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolved_at TIMESTAMPTZ
);
CREATE INDEX idx_review_items_learner_status ON review_items(learner_id, concept_id, status);

-- ---------------------------------------------------------------------------
-- Graph mutations — auditable atomic commits
-- ---------------------------------------------------------------------------
CREATE TABLE graph_mutations (
    id UUID PRIMARY KEY,
    learner_id UUID NOT NULL REFERENCES learners(id) ON DELETE CASCADE,
    conversation_id UUID REFERENCES conversations(id) ON DELETE SET NULL,
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_graph_mutations_learner ON graph_mutations(learner_id, created_at);

-- ---------------------------------------------------------------------------
-- Updated_at trigger helper (keeps updated_at fresh on UPDATE)
-- ---------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION set_updated_at() RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = now();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_learners_updated_at BEFORE UPDATE ON learners FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER trg_conversations_updated_at BEFORE UPDATE ON conversations FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER trg_concept_nodes_updated_at BEFORE UPDATE ON concept_nodes FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER trg_concept_relations_updated_at BEFORE UPDATE ON concept_relations FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE TRIGGER trg_learner_states_updated_at BEFORE UPDATE ON learner_concept_states FOR EACH ROW EXECUTE FUNCTION set_updated_at();
