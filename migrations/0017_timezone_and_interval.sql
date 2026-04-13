-- version: 29
-- description: Add timezone field to jobs for timezone-aware scheduling

ALTER TABLE jobs ADD COLUMN timezone TEXT;
