-- version: 26
-- description: Add expiry to API keys

ALTER TABLE api_keys ADD COLUMN expires_at TEXT;
