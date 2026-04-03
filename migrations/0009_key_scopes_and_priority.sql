-- version: 21
-- description: Add API key group scoping and job priority

-- API key group scoping: NULL means all groups (no restriction)
ALTER TABLE api_keys ADD COLUMN allowed_groups_json TEXT;

-- Job priority: higher priority jobs run first when multiple are scheduled
ALTER TABLE jobs ADD COLUMN priority INTEGER NOT NULL DEFAULT 0;
