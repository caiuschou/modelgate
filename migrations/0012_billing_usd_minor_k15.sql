-- k=15 fixed-point USD: balance_minor is signed integer string (1 USD = 10^15 minor units).
-- Migrates from balance_cents via REAL multiply (typical balances fit; very large cent balances may use approximation).

BEGIN IMMEDIATE;

ALTER TABLE user_balances RENAME TO user_balances_old;

CREATE TABLE user_balances (
    user_id INTEGER PRIMARY KEY NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    balance_minor TEXT NOT NULL
);

INSERT INTO user_balances (user_id, balance_minor)
SELECT user_id,
       printf('%.0f', CAST(balance_cents AS REAL) * 10000000000000.0)
FROM user_balances_old;

DROP TABLE user_balances_old;

ALTER TABLE billing_ledger RENAME TO billing_ledger_old;

-- Index name is global; drop before recreating the same name on the new table.
DROP INDEX IF EXISTS idx_billing_ledger_user_created;

CREATE TABLE billing_ledger (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at INTEGER NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('deposit', 'usage_charge')),
    amount_minor TEXT NOT NULL,
    balance_after_minor TEXT NOT NULL,
    request_id TEXT,
    model TEXT,
    prompt_tokens INTEGER,
    completion_tokens INTEGER,
    external_ref TEXT
);

CREATE INDEX idx_billing_ledger_user_created ON billing_ledger(user_id, created_at DESC);

INSERT INTO billing_ledger (
    id, user_id, created_at, kind, amount_minor, balance_after_minor,
    request_id, model, prompt_tokens, completion_tokens, external_ref
)
SELECT
    id,
    user_id,
    created_at,
    kind,
    printf('%.0f', CAST(amount_cents AS REAL) * 10000000000000.0),
    printf('%.0f', CAST(balance_after_cents AS REAL) * 10000000000000.0),
    request_id,
    model,
    prompt_tokens,
    completion_tokens,
    external_ref
FROM billing_ledger_old;

DROP TABLE billing_ledger_old;

COMMIT;
