-- version: 33
-- description: Add execution_id to events so UI can link directly to execution output

ALTER TABLE events ADD COLUMN execution_id TEXT;

CREATE INDEX IF NOT EXISTS idx_events_execution_id ON events(execution_id);
