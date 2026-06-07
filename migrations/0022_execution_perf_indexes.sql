-- version: 34
-- description: Add indexes on executions.created_at and executions.finished_at to keep the Runs page and retention purge fast at high row counts

CREATE INDEX IF NOT EXISTS idx_executions_created_at ON executions(created_at);

CREATE INDEX IF NOT EXISTS idx_executions_finished_at ON executions(finished_at);
