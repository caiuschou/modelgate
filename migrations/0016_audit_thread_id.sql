ALTER TABLE audit_logs ADD COLUMN thread_id TEXT;

CREATE INDEX IF NOT EXISTS idx_audit_logs_thread_id ON audit_logs (thread_id);
