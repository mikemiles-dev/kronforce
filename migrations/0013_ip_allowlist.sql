-- version: 25
-- description: Add IP allowlist to API keys

ALTER TABLE api_keys ADD COLUMN ip_allowlist TEXT;
