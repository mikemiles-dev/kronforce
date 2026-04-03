-- version: 18
-- description: Add secret flag to variables

ALTER TABLE variables ADD COLUMN secret INTEGER NOT NULL DEFAULT 0;
