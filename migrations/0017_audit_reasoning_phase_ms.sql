-- Streaming chat: wall-clock ms from first assistant reasoning delta to first non-empty content delta.
ALTER TABLE audit_logs ADD COLUMN reasoning_phase_ms INTEGER;
