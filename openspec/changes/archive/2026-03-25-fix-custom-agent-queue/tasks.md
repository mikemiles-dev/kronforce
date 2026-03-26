## 1. Database Migration and job_id in Queue

- [x] 1.1 Add migration version 8 in `src/db.rs` to `ALTER TABLE job_queue ADD COLUMN job_id TEXT`
- [x] 1.2 Update `enqueue_job()` signature to accept `job_id: Uuid` and include it in the INSERT statement
- [x] 1.3 Update `dequeue_job()` SELECT query to include `job_id` and add it to the returned JSON object
- [x] 1.4 Update the `enqueue_job()` call site in `src/executor.rs` to pass `job.id`

## 2. Stale Queue Cleanup Functions

- [x] 2.1 Add `fail_stale_pending_queue_items(max_age_secs: i64)` to `src/db.rs` — SELECT pending items older than max_age, UPDATE queue to completed, UPDATE execution to failed with timeout message
- [x] 2.2 Add `fail_stale_claimed_queue_items(max_age_secs: i64)` to `src/db.rs` — SELECT claimed items older than max_age, UPDATE queue to completed, UPDATE execution to failed with timeout message
- [x] 2.3 Call both cleanup functions from the agent health monitor loop in `src/bin/controller.rs` after `expire_agents()`

## 3. Frontend: Queued Badge

- [x] 3.1 Add CSS class `.badge-queued` with distinct styling (same color family as pending but different text)
- [x] 3.2 Update the `badge()` function or execution rendering to detect pending + custom agent and show "queued" badge
- [x] 3.3 Ensure `agent_id` is available in execution data returned by the API (verify existing fields)

## 4. Frontend: Pending Filter

- [x] 4.1 Add "Pending" status filter button to the executions action bar HTML using `execSearch.setStatusFilter(this, 'pending')`
- [x] 4.2 Verify the API correctly filters executions by `?status=pending`

## 5. Python Example Update

- [x] 5.1 Update `examples/custom_agent.py` to use `job["job_id"]` (with fallback) in the `report_result()` call
