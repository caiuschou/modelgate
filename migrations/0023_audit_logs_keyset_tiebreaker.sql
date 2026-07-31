-- Keyset/seek pagination for the request list orders by (created_at DESC, request_id DESC)
-- and seeks with WHERE (created_at < ? OR (created_at = ? AND request_id < ?)). Extend the
-- scope+time ordering indexes from 0022 with request_id as a trailing tiebreaker so the index
-- serves both the seek predicate and the full ORDER BY with no temp b-tree sort. request_id is
-- the primary key, giving a unique total order within an equal created_at second.
--
-- DROP + CREATE (not IF NOT EXISTS reuse) because 0022 may already have created these indexes
-- without the tiebreaker column; recreating is the only way to change an existing index's columns.
DROP INDEX IF EXISTS idx_audit_logs_user_created;
DROP INDEX IF EXISTS idx_audit_logs_team_created;
DROP INDEX IF EXISTS idx_audit_logs_token_created;

CREATE INDEX idx_audit_logs_user_created
    ON audit_logs (user_id, created_at DESC, request_id DESC);

CREATE INDEX idx_audit_logs_team_created
    ON audit_logs (team_id, created_at DESC, request_id DESC);

CREATE INDEX idx_audit_logs_token_created
    ON audit_logs (token_id, created_at DESC, request_id DESC);
