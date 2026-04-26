-- version: 31
-- description: Add optional expiration to variables

ALTER TABLE variables ADD COLUMN expires_at TEXT;
