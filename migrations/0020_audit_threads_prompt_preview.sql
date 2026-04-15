-- Last user-message preview for console session list (from chat completion request body).
ALTER TABLE audit_threads ADD COLUMN last_prompt_preview TEXT;
ALTER TABLE audit_threads ADD COLUMN last_prompt_at INTEGER;
