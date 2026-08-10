-- Materialize per-thread aggregates on audit_threads so the session-center default
-- view (no filters) reads one summary row per thread instead of GROUP BY-ing every
-- audit_logs row of the owner scope on each request. The GROUP BY path scaled with
-- total rows — a single long-lived session (tens of thousands of requests) kept the
-- endpoint at hundreds of ms locally / seconds on the deployed host.
--
-- Maintenance mirrors audit_hourly_rollups exactly (see apply_audit_thread_rollup):
-- skipped on insert for accept-phase and streaming rows, applied once when the row
-- becomes final (insert of a completed request, stream completion, or rejection).
-- audit_logs rows are never deleted, so increments never need reversal.

-- Team scope lists all members' threads (`team_scope = ?` without user_id), which the
-- 0019 index (user_id, team_scope, last_seen_at) cannot serve.
CREATE INDEX IF NOT EXISTS idx_audit_threads_team_last_seen
    ON audit_threads (team_scope, last_seen_at DESC);

ALTER TABLE audit_threads ADD COLUMN total_prompt_tokens INTEGER NOT NULL DEFAULT 0;
ALTER TABLE audit_threads ADD COLUMN total_completion_tokens INTEGER NOT NULL DEFAULT 0;
ALTER TABLE audit_threads ADD COLUMN total_tokens INTEGER NOT NULL DEFAULT 0;
ALTER TABLE audit_threads ADD COLUMN total_cached_prompt_tokens INTEGER NOT NULL DEFAULT 0;
ALTER TABLE audit_threads ADD COLUMN total_cost REAL NOT NULL DEFAULT 0;
ALTER TABLE audit_threads ADD COLUMN error_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE audit_threads ADD COLUMN total_latency_ms INTEGER NOT NULL DEFAULT 0;

-- Backfill every aggregate from the rows the console can actually see (the same
-- accept-phase guard query_audit_threads applies). Recomputing request_count and the
-- seen timestamps here also repairs any drift those counters accumulated before this
-- migration. Threads whose rows are all still hidden keep their existing counters.
UPDATE audit_threads SET
    request_count = g.cnt,
    first_seen_at = g.first_seen,
    last_seen_at = g.last_seen,
    total_prompt_tokens = g.pt,
    total_completion_tokens = g.ct,
    total_tokens = g.tt,
    total_cached_prompt_tokens = g.cpt,
    total_cost = g.cost,
    error_count = g.errors,
    total_latency_ms = g.latency
FROM (
    SELECT
        user_id AS u,
        COALESCE(team_id, 0) AS ts,
        trim(thread_id) AS tid,
        COUNT(*) AS cnt,
        MIN(created_at) AS first_seen,
        MAX(created_at) AS last_seen,
        COALESCE(SUM(prompt_tokens), 0) AS pt,
        COALESCE(SUM(completion_tokens), 0) AS ct,
        COALESCE(SUM(total_tokens), 0) AS tt,
        COALESCE(SUM(cached_prompt_tokens), 0) AS cpt,
        COALESCE(SUM(cost), 0.0) AS cost,
        SUM(CASE WHEN status_code IS NOT NULL AND status_code >= 400 THEN 1 ELSE 0 END) AS errors,
        COALESCE(SUM(COALESCE(latency_ms, 0)), 0) AS latency
    FROM audit_logs
    WHERE NOT (status_code IS NULL AND COALESCE(json_extract(metadata, '$.accept_phase'), 0) = 1)
      AND thread_id IS NOT NULL
      AND length(trim(thread_id)) > 0
    GROUP BY user_id, COALESCE(team_id, 0), trim(thread_id)
) AS g
WHERE audit_threads.user_id = g.u
  AND audit_threads.team_scope = g.ts
  AND audit_threads.thread_id = g.tid;
