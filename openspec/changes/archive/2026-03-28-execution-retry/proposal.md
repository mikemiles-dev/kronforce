## Why

When a job fails due to transient issues (network timeouts, temporary service unavailability, resource contention), operators must manually re-trigger it or write wrapper scripts with retry logic. Automatic retry with configurable backoff would handle the majority of transient failures without intervention, improving reliability for ETL pipelines, health checks, and deployment scripts.

## What Changes

- Add optional retry configuration to jobs: `max_retries` (default 0 = no retry), `retry_delay_secs` (initial delay between retries), and `retry_backoff` (multiplier for exponential backoff, default 1 = fixed delay)
- When an execution fails or times out, the system automatically schedules a retry if retries remain
- Each retry creates a new execution record linked to the original, with `triggered_by` set to `Retry`
- Retry count and attempt number are tracked on each execution so the UI can show "attempt 2/3"
- Retries stop on success, cancellation, or when max retries are exhausted
- A new `Retry` trigger source is added alongside the existing `Scheduler`, `Api`, and `Event` sources
- The execution detail view shows retry chain: which execution triggered this retry and how many attempts have been made
- Jobs can opt out of retry entirely (default behavior — backward compatible)

## Capabilities

### New Capabilities
- `execution-retry`: Automatic retry on job failure with configurable max retries, delay, and exponential backoff, including retry tracking on execution records and retry chain display in the UI

### Modified Capabilities

## Impact

- **Database**: New columns on `jobs` table (`max_retries`, `retry_delay_secs`, `retry_backoff`) via migration. New columns on `executions` table (`retry_of`, `attempt_number`) to track retry chains.
- **Backend**: `Job` struct gets retry config fields. `ExecutionRecord` gets retry tracking fields. `TriggerSource` enum gets a `Retry` variant. Post-execution logic in `executor/local.rs` and `api/callbacks.rs` gains retry scheduling. New retry delay logic using `tokio::time::sleep`.
- **Frontend**: Job create/edit modal gets retry config fields in the Advanced tab. Execution detail shows attempt number and retry chain.
- **No breaking changes**: Retry fields default to 0/disabled. Existing jobs and executions are unaffected.
