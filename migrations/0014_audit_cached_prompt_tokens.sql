-- Optional prompt cache breakdown (OpenAI-style usage.prompt_tokens_details.cached_tokens)

ALTER TABLE audit_logs ADD COLUMN cached_prompt_tokens INTEGER;

ALTER TABLE audit_hourly_rollups ADD COLUMN cached_prompt_tokens INTEGER NOT NULL DEFAULT 0;
