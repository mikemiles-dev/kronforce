-- version: 20
-- description: Add approval workflow support

ALTER TABLE jobs ADD COLUMN approval_required INTEGER NOT NULL DEFAULT 0;
