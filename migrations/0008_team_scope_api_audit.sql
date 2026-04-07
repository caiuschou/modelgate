ALTER TABLE api_keys ADD COLUMN team_id INTEGER REFERENCES teams(id) ON DELETE CASCADE;
CREATE INDEX idx_api_keys_team_id ON api_keys (team_id);

ALTER TABLE audit_logs ADD COLUMN team_id INTEGER REFERENCES teams(id) ON DELETE SET NULL;
CREATE INDEX idx_audit_logs_team_id ON audit_logs (team_id);
