ALTER TABLE api_keys ADD COLUMN name TEXT NOT NULL DEFAULT '';
ALTER TABLE api_keys ADD COLUMN description TEXT NOT NULL DEFAULT '';
ALTER TABLE api_keys ADD COLUMN disabled INTEGER NOT NULL DEFAULT 0;
ALTER TABLE api_keys ADD COLUMN last_used_at INTEGER;
ALTER TABLE api_keys ADD COLUMN expires_at INTEGER;
ALTER TABLE api_keys ADD COLUMN quota_monthly_tokens INTEGER;
ALTER TABLE api_keys ADD COLUMN quota_used_tokens INTEGER NOT NULL DEFAULT 0;
ALTER TABLE api_keys ADD COLUMN quota_period_start INTEGER;
ALTER TABLE api_keys ADD COLUMN model_allowlist TEXT;
ALTER TABLE api_keys ADD COLUMN ip_allowlist TEXT;

CREATE TABLE api_key_audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    key_id INTEGER NOT NULL,
    action TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    detail TEXT,
    FOREIGN KEY (user_id) REFERENCES users(id)
);
