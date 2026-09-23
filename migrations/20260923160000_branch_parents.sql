-- Branching: message tree parents for sibling navigation (Phase 5 fork).
-- Nullable: first message per conversation has no parent (NO_PARENT sentinel at the API layer).
-- Backfill sets each existing message's parent to its chronological predecessor so history
-- rendering is byte-identical to the previous synthesized chain.

ALTER TABLE conversation_messages ADD COLUMN parent_message_id TEXT REFERENCES conversation_messages(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_conversation_messages_parent ON conversation_messages(parent_message_id);

UPDATE conversation_messages SET parent_message_id = (
  SELECT prev_id FROM (
    SELECT id, LAG(id) OVER (PARTITION BY conversation_id ORDER BY created_at ASC, id ASC) AS prev_id
    FROM conversation_messages
  ) AS ordered WHERE ordered.id = conversation_messages.id
) WHERE parent_message_id IS NULL;
