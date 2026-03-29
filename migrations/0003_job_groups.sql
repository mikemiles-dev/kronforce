-- version: 15
-- description: Add group_name column to jobs

ALTER TABLE jobs ADD COLUMN group_name TEXT;
