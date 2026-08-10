-- Rebuild the 0022/0023 composite indexes as partial indexes embedding the
-- accept-phase visibility guard that audit_log_where_parts() appends to every
-- console query (list, count, threads, analytics fallback):
--
--     NOT (status_code IS NULL AND COALESCE(json_extract(metadata, '$.accept_phase'), 0) = 1)
--
-- The guard reads `metadata`, which no index contains, so the per-page COUNT(1)
-- and analytics aggregates had to fetch every matching row from the table just to
-- re-check it — O(rows per owner) row lookups on each request, seconds once an
-- owner accumulates millions of rows. With the guard as the index WHERE clause the
-- planner proves the predicate from the index itself: COUNT/aggregate scans become
-- index-only and the per-row json_extract disappears.
--
-- The index predicate text must stay byte-for-byte in sync with the guard emitted
-- by audit_log_where_parts(); SQLite only uses a partial index when the query's
-- WHERE terms match the index WHERE expression.
--
-- Column changes vs 0023:
--   * user index gains team_id so Personal scope (user_id = ? AND team_id IS NULL)
--     is fully covered — without it the count still fetched rows (measured 15s vs
--     0.3s over 3M synthetic rows).
--   * token index gains user_id + team_id so scope-qualified token filters stay
--     covering as well.

DROP INDEX IF EXISTS idx_audit_logs_user_created;
DROP INDEX IF EXISTS idx_audit_logs_team_created;
DROP INDEX IF EXISTS idx_audit_logs_token_created;

CREATE INDEX idx_audit_logs_user_created
    ON audit_logs (user_id, team_id, created_at DESC, request_id DESC)
    WHERE NOT (status_code IS NULL AND COALESCE(json_extract(metadata, '$.accept_phase'), 0) = 1);

CREATE INDEX idx_audit_logs_team_created
    ON audit_logs (team_id, created_at DESC, request_id DESC)
    WHERE NOT (status_code IS NULL AND COALESCE(json_extract(metadata, '$.accept_phase'), 0) = 1);

CREATE INDEX idx_audit_logs_token_created
    ON audit_logs (token_id, user_id, team_id, created_at DESC, request_id DESC)
    WHERE NOT (status_code IS NULL AND COALESCE(json_extract(metadata, '$.accept_phase'), 0) = 1);
