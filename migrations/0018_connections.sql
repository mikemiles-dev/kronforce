-- version: 30
-- description: Add named connections for credentials management

CREATE TABLE IF NOT EXISTS connections (
    name        TEXT PRIMARY KEY NOT NULL,
    conn_type   TEXT NOT NULL,
    description TEXT,
    config      TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
