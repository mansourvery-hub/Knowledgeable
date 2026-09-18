-- M8: Personal Knowledge Wiki cache (data_models §21).
-- A persistent human-readable projection of the learner graph, maintained
-- with low frequency. One row per learner/concept; anchors embed JSON.

CREATE TABLE IF NOT EXISTS concept_wiki_pages (
    id TEXT PRIMARY KEY NOT NULL,
    learner_id TEXT NOT NULL REFERENCES learners(id) ON DELETE CASCADE,
    concept_id TEXT NOT NULL REFERENCES concept_nodes(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    summary TEXT NOT NULL,
    personalized_content TEXT NOT NULL,
    known_prerequisites TEXT NOT NULL CHECK (json_valid(known_prerequisites)),
    related_concepts TEXT NOT NULL CHECK (json_valid(related_concepts)),
    learner_confidence_at_generation REAL NOT NULL CHECK (learner_confidence_at_generation >= 0.0 AND learner_confidence_at_generation <= 1.0),
    version INTEGER NOT NULL DEFAULT 1,
    is_stale INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    UNIQUE(learner_id, concept_id)
) STRICT;

CREATE INDEX IF NOT EXISTS idx_wiki_pages_learner_concept ON concept_wiki_pages(learner_id, concept_id);
