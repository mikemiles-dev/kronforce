-- version: 24
-- description: Add default job templates

INSERT OR IGNORE INTO job_templates (name, description, snapshot_json, created_by, created_at) VALUES
('HTTP Health Check', 'Monitor a URL endpoint every 5 minutes with failure alerts', '{"task":{"type":"http","method":"GET","url":"https://example.com/health","expect_status":200},"notifications":{"on_failure":true,"on_success":false,"on_assertion_failure":false},"group":"Monitoring","retry_max":2,"retry_delay_secs":10,"retry_backoff":2.0}', 'system', '2026-04-03T00:00:00Z'),

('Shell Cron Task', 'Run a shell command on a cron schedule', '{"task":{"type":"shell","command":"echo \"Hello from Kronforce\""},"group":"Default"}', 'system', '2026-04-03T00:00:00Z'),

('ETL Extract', 'Extract data with output capture and variable write-back', '{"task":{"type":"shell","command":"python3 extract.py"},"group":"ETL","output_rules":{"extractions":[{"name":"record_count","pattern":"Extracted (\\d+) records","type":"regex","write_to_variable":"LAST_EXTRACT_COUNT","target":"variable"}],"assertions":[{"pattern":"complete","message":"Extract did not complete successfully"}],"triggers":[]},"notifications":{"on_failure":true,"on_success":false,"on_assertion_failure":true}}', 'system', '2026-04-03T00:00:00Z'),

('Deploy with Approval', 'Production deployment requiring approval before execution', '{"task":{"type":"shell","command":"./deploy.sh production"},"group":"Deploys","approval_required":true,"retry_max":1,"retry_delay_secs":30,"retry_backoff":1.0,"notifications":{"on_failure":true,"on_success":true,"on_assertion_failure":false},"sla_deadline":"06:00","sla_warning_mins":15}', 'system', '2026-04-03T00:00:00Z'),

('Database Backup', 'Nightly database backup with failure notifications', '{"task":{"type":"shell","command":"pg_dump -U postgres mydb | gzip > /backup/db-$(date +%Y%m%d).sql.gz"},"group":"Maintenance","notifications":{"on_failure":true,"on_success":false,"on_assertion_failure":false},"timeout_secs":3600}', 'system', '2026-04-03T00:00:00Z'),

('API Latency Test', 'HTTP request with response time extraction', '{"task":{"type":"http","method":"GET","url":"https://api.example.com/ping"},"group":"Monitoring","output_rules":{"extractions":[],"assertions":[],"triggers":[{"pattern":"timeout|error","severity":"error"}]}}', 'system', '2026-04-03T00:00:00Z'),

('Event Reactor', 'React to job failures with a cleanup or notification script', '{"task":{"type":"shell","command":"echo \"Handling failure for job: $1\""},"group":"Default","notifications":{"on_failure":true,"on_success":false,"on_assertion_failure":false}}', 'system', '2026-04-03T00:00:00Z'),

('File Transfer (SFTP)', 'Upload a file to a remote server via SFTP', '{"task":{"type":"ftp","protocol":"sftp","host":"sftp.example.com","port":22,"remote_path":"/uploads/data.csv","local_path":"/data/export.csv","direction":"upload","username":"deploy"},"group":"Default","retry_max":2,"retry_delay_secs":15,"retry_backoff":2.0}', 'system', '2026-04-03T00:00:00Z');
