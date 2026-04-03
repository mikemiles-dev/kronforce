-- version: 19
-- description: Add job version history

CREATE TABLE IF NOT EXISTS job_versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    snapshot_json TEXT NOT NULL,
    changed_by_key_id TEXT,
    changed_by_name TEXT,
    created_at TEXT NOT NULL,
    UNIQUE(job_id, version)
);

CREATE INDEX IF NOT EXISTS idx_job_versions_job_id ON job_versions(job_id);
