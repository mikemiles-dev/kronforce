-- version: 23
-- description: Add job templates table

CREATE TABLE IF NOT EXISTS job_templates (
    name TEXT PRIMARY KEY,
    description TEXT,
    snapshot_json TEXT NOT NULL,
    created_by TEXT,
    created_at TEXT NOT NULL
);
