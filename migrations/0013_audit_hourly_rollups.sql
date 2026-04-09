-- Pre-aggregated hourly audit metrics for scalable analytics reads.
-- scope: 'p' = personal (scope_id = user_id), 't' = team (scope_id = team_id)

CREATE TABLE audit_hourly_rollups (
    bucket_start INTEGER NOT NULL,
    scope TEXT NOT NULL CHECK (scope IN ('p', 't')),
    scope_id INTEGER NOT NULL,
    request_count INTEGER NOT NULL DEFAULT 0,
    success_count INTEGER NOT NULL DEFAULT 0,
    prompt_tokens INTEGER NOT NULL DEFAULT 0,
    completion_tokens INTEGER NOT NULL DEFAULT 0,
    total_tokens INTEGER NOT NULL DEFAULT 0,
    cost_sum REAL NOT NULL DEFAULT 0,
    latency_ms_sum INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (bucket_start, scope, scope_id)
);

CREATE INDEX idx_audit_hourly_rollups_scope_time
    ON audit_hourly_rollups (scope, scope_id, bucket_start);
