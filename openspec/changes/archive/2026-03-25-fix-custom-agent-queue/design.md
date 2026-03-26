## Context

Custom agents use a pull-based queue: the controller enqueues jobs in a `job_queue` SQLite table, and agents poll `GET /api/agent-queue/{agent_id}/next` to claim work. Results are reported back via `POST /api/callbacks/execution-result`.

Current issues:
- `job_queue` table has no `job_id` column — agents can't reference the originating job
- No timeout or cleanup for stale queue items (pending forever, claimed but never completed)
- UI shows "pending" badge for queued executions with no indication they're waiting for a custom agent to poll
- No "Pending" filter button in the executions action bar

The scheduler runs a tick loop (configurable interval, default 1s). The controller also runs a 10-second agent health monitor loop that expires offline agents.

## Goals / Non-Goals

**Goals:**
- Include `job_id` in queue data and dequeue response
- Fail executions that are stuck in the queue (unclaimed or abandoned)
- Show clear UI indication when an execution is queued for a custom agent
- Add "Pending" filter to executions page

**Non-Goals:**
- Changing the pull-based architecture to push
- Adding retry/requeue logic for failed custom agent jobs
- Adding a dedicated "queued" execution status enum variant (too invasive — use UI-side detection instead)

## Decisions

### 1. Add `job_id` column via versioned migration

**Decision**: Add migration (version 8) to `ALTER TABLE job_queue ADD COLUMN job_id TEXT`. Update `enqueue_job()` to accept and store `job_id`, and include it in the `dequeue_job()` JSON response.

**Rationale**: This is the simplest fix. The column is nullable for backward compatibility with any existing queue rows. The Python example already does `job.get("job_id", "")` so it will pick up the new field automatically.

**Alternative considered**: Embedding `job_id` inside `task_json` — rejected because it's not a property of the task, it's a property of the queue entry.

### 2. Stale queue cleanup in the agent health monitor loop

**Decision**: Add queue cleanup to the existing 10-second agent health monitor loop in `controller.rs`, not the scheduler. Add two cleanup operations to `db.rs`:

1. **`fail_stale_pending_queue_items(max_age_secs)`**: Find queue items with `status = 'pending'` where `created_at` is older than `max_age_secs` (default: 300 seconds / 5 minutes). Mark those queue items as `completed` and update the corresponding execution to `Failed` with stderr `"queued for custom agent but never claimed (timeout)"`.

2. **`fail_stale_claimed_queue_items(max_age_secs)`**: Find queue items with `status = 'claimed'` where `claimed_at` is older than `max_age_secs` (default: 600 seconds / 10 minutes). Mark as `completed` and update execution to `Failed` with stderr `"custom agent claimed job but never reported result (timeout)"`.

**Rationale**: The health monitor already runs every 10 seconds and deals with agent state. Queue cleanup is conceptually similar (detecting unresponsive agents). Adding it to the scheduler tick would work but is less appropriate since the scheduler deals with job firing, not agent health.

**Alternative considered**: A separate spawned task — rejected because it adds unnecessary complexity when the health monitor loop is already there.

### 3. UI "queued" indicator via badge logic, not new status enum

**Decision**: In the frontend `badge()` function and execution detail view, detect when an execution is `pending` AND was dispatched to a custom agent. Show a "queued" badge (styled like "pending" but with distinct text) instead of just "pending".

Implementation approach: The execution record already stores `agent_id`. The agent list (already cached client-side in `allAgents`) includes `agent_type`. If an execution has status `pending` and its `agent_id` resolves to a custom agent, display "queued" instead of "pending".

**Rationale**: Adding a new `ExecutionStatus::Queued` variant would require changes across models, database, API serialization, and all status filtering — disproportionate to the benefit. A frontend-only detection is simpler and achieves the same UX goal.

### 4. Add "Pending" filter button to executions action bar

**Decision**: Add a "Pending" status filter button to the executions action bar, using the `execSearch` factory instance. This filters by `status=pending` on the API side.

**Rationale**: Users need to be able to find stuck jobs. This is a one-line HTML addition plus the API already supports `?status=pending` filtering.

## Risks / Trade-offs

- **Stale cleanup timeout values are hardcoded** → Acceptable for now; can be made configurable later via environment variables if needed.
- **Frontend "queued" detection relies on agent list being loaded** → The agent list is fetched on page load and cached. If agents haven't loaded yet, "pending" is shown as fallback — acceptable degradation.
- **Migration adds nullable column** → Old queue rows (if any) will have `NULL` job_id. The Python agent already handles missing `job_id` with a default empty string.
- **Health monitor loop does more work** → Two extra SQL queries every 10 seconds is negligible on SQLite.
