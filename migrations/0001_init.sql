-- version: 13
-- description: Initial schema (v0.1.0)

CREATE TABLE IF NOT EXISTS jobs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    command TEXT,
    task_json TEXT,
    run_as TEXT,
    created_by TEXT,
    schedule_json TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'scheduled',
    timeout_secs INTEGER,
    depends_on_json TEXT NOT NULL DEFAULT '[]',
    target_json TEXT,
    output_rules_json TEXT,
    notifications_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS executions (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL REFERENCES jobs(id),
    agent_id TEXT,
    task_snapshot_json TEXT,
    status TEXT NOT NULL,
    exit_code INTEGER,
    stdout TEXT NOT NULL DEFAULT '',
    stderr TEXT NOT NULL DEFAULT '',
    stdout_truncated INTEGER NOT NULL DEFAULT 0,
    stderr_truncated INTEGER NOT NULL DEFAULT 0,
    started_at TEXT,
    finished_at TEXT,
    triggered_by_json TEXT NOT NULL,
    extracted_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_executions_job_id ON executions(job_id);
CREATE INDEX IF NOT EXISTS idx_executions_status ON executions(status);
CREATE INDEX IF NOT EXISTS idx_executions_started_at ON executions(started_at);

CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    tags_json TEXT NOT NULL DEFAULT '[]',
    hostname TEXT NOT NULL,
    address TEXT NOT NULL,
    port INTEGER NOT NULL,
    agent_type TEXT DEFAULT 'standard',
    status TEXT NOT NULL DEFAULT 'online',
    last_heartbeat TEXT,
    registered_at TEXT NOT NULL,
    task_types_json TEXT
);

CREATE TABLE IF NOT EXISTS events (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    severity TEXT NOT NULL DEFAULT 'info',
    message TEXT NOT NULL,
    job_id TEXT,
    agent_id TEXT,
    api_key_id TEXT,
    api_key_name TEXT,
    details TEXT,
    timestamp TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);

CREATE TABLE IF NOT EXISTS api_keys (
    id TEXT PRIMARY KEY,
    key_prefix TEXT NOT NULL,
    key_hash TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'viewer',
    created_at TEXT NOT NULL,
    last_used_at TEXT,
    active INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS job_queue (
    id TEXT PRIMARY KEY,
    execution_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    job_id TEXT,
    task_json TEXT NOT NULL,
    run_as TEXT,
    timeout_secs INTEGER,
    callback_url TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL,
    claimed_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_job_queue_agent ON job_queue(agent_id, status);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT OR IGNORE INTO settings (key, value) VALUES ('retention_days', '7');

CREATE TABLE IF NOT EXISTS variables (
    name TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
