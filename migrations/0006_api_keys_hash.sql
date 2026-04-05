PRAGMA foreign_keys=OFF;

CREATE TABLE api_keys_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    api_key TEXT UNIQUE,
    api_key_hash TEXT UNIQUE,
    key_preview TEXT NOT NULL DEFAULT '',
    created_at INTEGER NOT NULL,
    revoked INTEGER NOT NULL DEFAULT 0,
    name TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    disabled INTEGER NOT NULL DEFAULT 0,
    last_used_at INTEGER,
    expires_at INTEGER,
    quota_monthly_tokens INTEGER,
    quota_used_tokens INTEGER NOT NULL DEFAULT 0,
    quota_period_start INTEGER,
    model_allowlist TEXT,
    ip_allowlist TEXT,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

INSERT INTO api_keys_new (
    id, user_id, api_key, api_key_hash, key_preview, created_at, revoked,
    name, description, disabled, last_used_at, expires_at,
    quota_monthly_tokens, quota_used_tokens, quota_period_start, model_allowlist, ip_allowlist
)
SELECT
    id, user_id, api_key, NULL, '', created_at, revoked,
    name, description, disabled, last_used_at, expires_at,
    quota_monthly_tokens, quota_used_tokens, quota_period_start, model_allowlist, ip_allowlist
FROM api_keys;

DROP TABLE api_keys;
ALTER TABLE api_keys_new RENAME TO api_keys;

CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys(user_id);

PRAGMA foreign_keys=ON;
