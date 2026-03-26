## Why

Custom agent job execution has multiple bugs and UX gaps that make it unreliable and confusing. Jobs sent to custom agents can silently get stuck in `pending` status forever if the agent crashes or never polls. The UI provides no indication that a job is queued for a custom agent vs simply pending, and the queue data omits `job_id` — causing the Python agent to send empty strings in callbacks. There is no cleanup mechanism for stale queue items and no way to filter pending executions in the UI.

## What Changes

- **Include `job_id` in queue data**: Add `job_id` to the `job_queue` table schema and include it in the dequeue response so custom agents can reference the originating job
- **Add queue timeout and stale cleanup**: Implement a periodic check that marks queued executions as `failed` if unclaimed after a configurable timeout, and requeues or fails items that were claimed but never completed
- **Add "queued" execution status visibility**: Show a distinct "queued" badge in the UI for executions dispatched to custom agents that are waiting for the agent to poll, differentiating from other pending states
- **Add "Pending" filter to executions UI**: Add a status filter button for pending/queued executions so users can find stuck jobs
- **Update Python example**: Include `job_id` from queue response in the callback payload

## Capabilities

### New Capabilities
- `custom-agent-queue`: Queue reliability, timeout handling, stale cleanup, and UI visibility for custom agent job dispatch

### Modified Capabilities

## Impact

- **Backend**: `src/db.rs` (migration for `job_id` column in `job_queue`, dequeue query), `src/executor.rs` (enqueue call), `src/api.rs` (queue polling response)
- **Scheduler**: New periodic task in the scheduler loop to clean up stale queue items
- **Frontend**: `src/dashboard.html` (queued badge styling, pending filter button)
- **Example**: `examples/custom_agent.py` (use `job_id` from queue response)
- **Database**: Migration adds `job_id` column to `job_queue` table
