-- Phase 5 (pin/archive): learner conversation flags. Defaults keep every
-- existing conversation unpinned and unarchived; no backfill needed.

ALTER TABLE conversations ADD COLUMN is_archived INTEGER NOT NULL DEFAULT 0;
ALTER TABLE conversations ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;
