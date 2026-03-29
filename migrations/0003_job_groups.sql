-- version: 15
-- description: Add group_name column to jobs

ALTER TABLE jobs ADD COLUMN group_name TEXT DEFAULT 'Default';
UPDATE jobs SET group_name = 'Default' WHERE group_name IS NULL;
