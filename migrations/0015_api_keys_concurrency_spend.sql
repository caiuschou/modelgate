ALTER TABLE api_keys ADD COLUMN max_concurrent_requests INTEGER;
ALTER TABLE api_keys ADD COLUMN quota_monthly_spend_minor TEXT;
ALTER TABLE api_keys ADD COLUMN quota_used_spend_minor TEXT NOT NULL DEFAULT '0';
ALTER TABLE api_keys ADD COLUMN quota_spend_period_start INTEGER;
