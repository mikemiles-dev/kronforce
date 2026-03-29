-- version: 16
-- description: Add retry config to jobs and retry tracking to executions

ALTER TABLE jobs ADD COLUMN retry_max INTEGER DEFAULT 0;
ALTER TABLE jobs ADD COLUMN retry_delay_secs INTEGER DEFAULT 0;
ALTER TABLE jobs ADD COLUMN retry_backoff REAL DEFAULT 1.0;

ALTER TABLE executions ADD COLUMN retry_of TEXT;
ALTER TABLE executions ADD COLUMN attempt_number INTEGER DEFAULT 1;
