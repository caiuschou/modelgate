CREATE TABLE IF NOT EXISTS byok_profiles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    owner_user_id INTEGER,
    owner_team_id INTEGER,
    name TEXT NOT NULL DEFAULT '',
    base_url TEXT NOT NULL,
    api_key_ciphertext BLOB NOT NULL,
    api_key_nonce BLOB NOT NULL,
    key_version INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    revoked_at INTEGER,
    CHECK (
        (owner_user_id IS NOT NULL AND owner_team_id IS NULL)
        OR (owner_user_id IS NULL AND owner_team_id IS NOT NULL)
    ),
    FOREIGN KEY (owner_user_id) REFERENCES users(id),
    FOREIGN KEY (owner_team_id) REFERENCES teams(id)
);

CREATE INDEX IF NOT EXISTS idx_byok_profiles_owner_user
    ON byok_profiles(owner_user_id) WHERE revoked_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_byok_profiles_owner_team
    ON byok_profiles(owner_team_id) WHERE revoked_at IS NULL;
