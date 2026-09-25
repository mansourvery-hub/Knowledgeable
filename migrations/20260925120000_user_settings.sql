-- Per-learner UI settings blob store (M1 pinned-order persistence).
-- Single row per key; values are JSON encoded by the service layer.
CREATE TABLE IF NOT EXISTS user_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
