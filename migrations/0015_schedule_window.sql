-- version: 27
-- description: Add schedule window (starts_at, expires_at) to jobs

ALTER TABLE jobs ADD COLUMN starts_at TEXT;
ALTER TABLE jobs ADD COLUMN expires_at TEXT;
