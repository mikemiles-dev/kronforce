-- version: 22
-- description: Add SLA deadline tracking to jobs

ALTER TABLE jobs ADD COLUMN sla_deadline TEXT;
ALTER TABLE jobs ADD COLUMN sla_warning_mins INTEGER NOT NULL DEFAULT 0;
