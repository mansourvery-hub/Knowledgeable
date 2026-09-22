-- Phase 5 (bookmarks): learner conversation tags + conversation membership.
-- Tags are learner-scoped (single-user: the default learner); mappings die
-- with their conversation via FK cascade, and with their tag the same way.

CREATE TABLE IF NOT EXISTS conversation_tags (
    tag TEXT NOT NULL,
    learner_id TEXT NOT NULL REFERENCES learners(id) ON DELETE CASCADE,
    description TEXT,
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    PRIMARY KEY (tag, learner_id)
) STRICT;

CREATE TABLE IF NOT EXISTS conversation_tag_map (
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    tag TEXT NOT NULL,
    learner_id TEXT NOT NULL,
    PRIMARY KEY (conversation_id, tag),
    -- Deferrable so renames can update both sides inside one transaction.
    FOREIGN KEY (tag, learner_id) REFERENCES conversation_tags(tag, learner_id) ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED
) STRICT;

CREATE INDEX IF NOT EXISTS idx_tag_map_conversation ON conversation_tag_map(conversation_id);
CREATE INDEX IF NOT EXISTS idx_tag_map_tag_learner ON conversation_tag_map(tag, learner_id);
