-- version: 32
-- description: Add system_info JSON to agents for node inventory

ALTER TABLE agents ADD COLUMN system_info_json TEXT;
