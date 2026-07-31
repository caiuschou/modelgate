-- Composite indexes for the console audit-log listing / session-center / analytics
-- queries. They all filter by owner scope (user_id / team_id / token_id) plus a
-- created_at range and ORDER BY created_at DESC. With only single-column indexes,
-- SQLite had to either sort large per-owner result sets or scan the global
-- created_at index re-checking user_id row by row. These composites serve the
-- filter and the ordering from one index and turn the per-request COUNT(1) into a
-- cheap index range scan.

CREATE INDEX IF NOT EXISTS idx_audit_logs_user_created
    ON audit_logs (user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_audit_logs_team_created
    ON audit_logs (team_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_audit_logs_token_created
    ON audit_logs (token_id, created_at DESC);

-- Session-center thread grouping filters by owner scope then GROUPs by thread_id.
CREATE INDEX IF NOT EXISTS idx_audit_logs_user_thread
    ON audit_logs (user_id, team_id, thread_id);

-- Superseded by the composite prefixes above; dropping them cuts write amplification.
DROP INDEX IF EXISTS idx_audit_logs_user_id;
DROP INDEX IF EXISTS idx_audit_logs_team_id;
DROP INDEX IF EXISTS idx_audit_logs_token_id;
