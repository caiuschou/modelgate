-- Session-level upstream affinity: ordered pool + RR cursor + per-session bindings.

ALTER TABLE api_keys ADD COLUMN session_affinity_enabled INTEGER NOT NULL DEFAULT 0;
ALTER TABLE api_keys ADD COLUMN upstream_pool_json TEXT;
ALTER TABLE api_keys ADD COLUMN session_rr_cursor INTEGER NOT NULL DEFAULT 0;

CREATE TABLE session_upstream_bindings (
    api_key_id INTEGER NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    session_key TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('platform', 'byok')),
    byok_profile_id INTEGER,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (api_key_id, session_key)
);

CREATE INDEX idx_session_upstream_bindings_key ON session_upstream_bindings(api_key_id);
