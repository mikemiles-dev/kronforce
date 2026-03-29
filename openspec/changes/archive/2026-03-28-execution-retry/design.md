## Context

When a job execution completes, the post-execution flow runs in `Executor::handle_execution_complete()` (local) or in the callback handler at `api/callbacks.rs` (remote agent). Both paths update the execution record, run output rules, send notifications, and emit events. Retry logic needs to hook into this flow — after determining the execution failed, check if retries are configured, and if so, schedule a new execution after the configured delay.

The `TriggerSource` enum currently has variants: `Scheduler`, `Api`, `Event { event_id }`, and `Dependency`. A `Retry` variant needs to be added to track retry provenance.

Execution records are stored in the `executions` table. The `triggered_by_json` column stores the `TriggerSource` as JSON. There is no current concept of linking one execution to another.

## Goals / Non-Goals

**Goals:**
- Automatic retry on failure/timeout with configurable count, delay, and backoff
- Retry chain tracking so operators can see "this is attempt 3/5, retrying execution X"
- Works for both local and agent-dispatched executions
- Exponential backoff to avoid hammering failing services
- No retry on success, cancellation, or assertion failures (assertion = logic error, not transient)

**Non-Goals:**
- Retry with different parameters (e.g., different timeout or different agent)
- Conditional retry based on exit code or output content
- Retry across job restarts (if controller restarts mid-retry-chain, pending retries are lost)
- Retry queue persistence — retries are scheduled via `tokio::spawn` with sleep, not stored in DB
- Circuit breaker pattern (stop retrying after N consecutive failures across different executions)

## Decisions

### 1. Retry config as three fields on the Job struct

```rust
pub retry_max: u32,           // 0 = no retry (default)
pub retry_delay_secs: u64,    // initial delay between retries (default 0)
pub retry_backoff: f64,        // multiplier per attempt (default 1.0 = fixed delay)
```

Stored as three columns on the `jobs` table. The delay for attempt N is: `retry_delay_secs * retry_backoff^(attempt - 1)`.

**Alternatives considered:**
- Single `retry_policy` JSON column: Harder to query and validate. Three simple columns are clearer.
- Retry config as a nested struct: Over-abstracted for three fields. Keep it flat.

### 2. Retry tracking via `retry_of` and `attempt_number` on executions

```sql
ALTER TABLE executions ADD COLUMN retry_of TEXT;       -- UUID of the original execution
ALTER TABLE executions ADD COLUMN attempt_number INTEGER DEFAULT 1;
```

- `retry_of` points to the **original** (first) execution in the chain, not the immediately preceding one. This makes it easy to query "show me all attempts for this execution."
- `attempt_number` starts at 1 for the first run, 2 for first retry, etc.
- The `TriggerSource::Retry` variant carries `original_execution_id` and `attempt`.

### 3. Retry scheduling via `tokio::spawn` with sleep

After a failed execution, if the job has `retry_max > 0` and `attempt_number < retry_max + 1`:

```rust
let delay = retry_delay_secs * retry_backoff.powi(attempt - 1);
tokio::spawn(async move {
    tokio::time::sleep(Duration::from_secs(delay as u64)).await;
    executor.execute(&job, TriggerSource::Retry { ... }, callback_url).await;
});
```

**Alternatives considered:**
- Scheduler-based retry (store retry schedule in DB, let scheduler pick it up): More robust to restarts but adds significant complexity. The simple spawn+sleep approach handles 99% of cases.
- Retry queue table: Overkill — retries are rare and short-lived.

### 4. Retry decision point in `handle_execution_complete`

The retry check runs after output rules and notifications but before emitting the completion event. This means:
- Notifications fire on each failed attempt (operator sees "attempt 2 failed")
- Output rules run on each attempt (extractions from partial success)
- The final completion event is emitted whether or not a retry is scheduled

### 5. No retry on assertion failure or cancellation

Only `ExecutionStatus::Failed` and `ExecutionStatus::TimedOut` trigger retries. Assertion failures are logic errors (output didn't match), not transient. Cancellations are explicit user actions.

### 6. Max delay cap at 1 hour

Regardless of backoff calculation, the delay between retries is capped at 3600 seconds (1 hour) to prevent runaway exponential backoff.

## Risks / Trade-offs

- **Retries lost on controller restart** → Spawned tokio tasks don't survive process restarts. Acceptable for now. If a retry is in-flight when the controller dies, it's lost. The operator can manually re-trigger.
- **Concurrent retries for same job** → If a cron-triggered execution and a retry-triggered execution overlap, they run concurrently. This is by design — the scheduler doesn't know about retries. Jobs that can't handle concurrent runs should use dependencies or manual triggers.
- **Notification spam** → Each failed attempt sends a notification. This is intentional — operators want to know about each failure. A "notify only on final failure" option could be added later.
- **Agent retries** → For agent-dispatched jobs, the retry re-dispatches to the same agent targeting strategy (any, tagged, specific). The retry might land on a different agent than the original. This is correct — if the original agent is having issues, a different one might succeed.
