-- One row per (API key owner scope, normalized thread id). `team_scope` 0 means personal (`audit_logs.team_id` NULL).
CREATE TABLE audit_threads (
    user_id INTEGER NOT NULL,
    team_scope INTEGER NOT NULL,
    thread_id TEXT NOT NULL,
    first_seen_at INTEGER NOT NULL,
    last_seen_at INTEGER NOT NULL,
    request_count INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (user_id, team_scope, thread_id)
);

CREATE INDEX idx_audit_threads_user_team_last_seen ON audit_threads (user_id, team_scope, last_seen_at DESC);

-- Backfill from existing audit rows (trim thread_id to align with gateway normalization).
INSERT INTO audit_threads (user_id, team_scope, thread_id, first_seen_at, last_seen_at, request_count)
SELECT
    user_id,
    COALESCE(team_id, 0),
    trim(thread_id),
    MIN(created_at),
    MAX(created_at),
    COUNT(*)
FROM audit_logs
WHERE user_id IS NOT NULL
  AND thread_id IS NOT NULL
  AND length(trim(thread_id)) > 0
GROUP BY user_id, COALESCE(team_id, 0), trim(thread_id);
