ALTER TABLE audit_logs ADD COLUMN app_id TEXT;
ALTER TABLE audit_logs ADD COLUMN finish_reason TEXT;

CREATE INDEX IF NOT EXISTS idx_audit_logs_app_id ON audit_logs (app_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_finish_reason ON audit_logs (finish_reason);
