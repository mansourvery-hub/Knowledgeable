-- Phase 5 (minimal share, F27): public share links for conversations.
-- One row per link; share ids are unguessable UUIDs (never the
-- conversation id). Messages are read live (no snapshots); deleting the
-- conversation cascades the link. No expiry: revocation is explicit delete.

CREATE TABLE IF NOT EXISTS shared_links (
    share_id TEXT PRIMARY KEY NOT NULL,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    learner_id TEXT NOT NULL REFERENCES learners(id) ON DELETE CASCADE,
    title TEXT,
    target_message_id TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
) STRICT;

CREATE INDEX IF NOT EXISTS idx_shared_links_conversation ON shared_links(conversation_id);
