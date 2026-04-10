-- version: 28
-- description: Add concurrency controls, parameter schemas, webhook tokens, and execution params

ALTER TABLE jobs ADD COLUMN max_concurrent INTEGER DEFAULT 0;
ALTER TABLE jobs ADD COLUMN parameters_json TEXT;
ALTER TABLE jobs ADD COLUMN webhook_token TEXT UNIQUE;
ALTER TABLE executions ADD COLUMN params_json TEXT;
CREATE INDEX IF NOT EXISTS idx_jobs_webhook_token ON jobs(webhook_token);
