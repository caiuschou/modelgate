CREATE TABLE audit_logs (
    request_id TEXT PRIMARY KEY,
    user_id INTEGER,
    token_id INTEGER,
    channel_id TEXT,
    model TEXT,
    request_type TEXT,
    request_body_path TEXT,
    response_body_path TEXT,
    status_code INTEGER,
    error_message TEXT,
    prompt_tokens INTEGER,
    completion_tokens INTEGER,
    total_tokens INTEGER,
    cost REAL,
    latency_ms INTEGER,
    metadata TEXT,
    created_at INTEGER NOT NULL
);

CREATE INDEX idx_audit_logs_created_at ON audit_logs (created_at);
CREATE INDEX idx_audit_logs_user_id ON audit_logs (user_id);
CREATE INDEX idx_audit_logs_token_id ON audit_logs (token_id);
