-- Password for console login / registration; NULL for legacy users created via POST /users only.
ALTER TABLE users ADD COLUMN password_hash TEXT;
