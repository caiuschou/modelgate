-- USD balance in integer cents. Ledger rows: deposit positive amount_cents; usage_charge negative amount_cents.

CREATE TABLE user_balances (
    user_id INTEGER PRIMARY KEY NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    balance_cents INTEGER NOT NULL DEFAULT 0 CHECK (balance_cents >= 0)
);

CREATE TABLE billing_ledger (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at INTEGER NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('deposit', 'usage_charge')),
    amount_cents INTEGER NOT NULL,
    balance_after_cents INTEGER NOT NULL,
    request_id TEXT,
    model TEXT,
    prompt_tokens INTEGER,
    completion_tokens INTEGER,
    external_ref TEXT
);

CREATE INDEX idx_billing_ledger_user_created ON billing_ledger(user_id, created_at DESC);

INSERT INTO user_balances (user_id, balance_cents)
SELECT id, 0 FROM users;
