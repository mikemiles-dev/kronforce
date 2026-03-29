## ADDED Requirements

### Requirement: Retry configuration on jobs
The `Job` model SHALL have three optional retry configuration fields: `retry_max` (u32, default 0), `retry_delay_secs` (u64, default 0), and `retry_backoff` (f64, default 1.0). These fields SHALL be stored in the `jobs` table via migration and included in create/update API requests.

#### Scenario: Create job with retry config
- **WHEN** a user creates a job with `"retry_max": 3, "retry_delay_secs": 10, "retry_backoff": 2.0`
- **THEN** the job is created with those retry settings and they appear in the job response

#### Scenario: Create job without retry config
- **WHEN** a user creates a job without retry fields
- **THEN** the job defaults to `retry_max: 0`, `retry_delay_secs: 0`, `retry_backoff: 1.0` (no retries)

#### Scenario: Update retry config
- **WHEN** a user updates a job with `"retry_max": 5`
- **THEN** the job's retry_max is updated to 5

#### Scenario: Existing jobs unaffected by migration
- **WHEN** the migration adds retry columns
- **THEN** all existing jobs have `retry_max` defaulting to 0 (no retries enabled)

### Requirement: Automatic retry on failure
The system SHALL automatically retry a job execution when it finishes with status `Failed` or `TimedOut`, if the job has `retry_max > 0` and the current attempt number is less than or equal to `retry_max`.

#### Scenario: First failure triggers retry
- **WHEN** a job with `retry_max: 3` fails on its first execution (attempt 1)
- **THEN** the system schedules a retry execution after `retry_delay_secs` seconds

#### Scenario: Retry succeeds
- **WHEN** a retry execution (attempt 2) succeeds
- **THEN** no further retries are scheduled

#### Scenario: All retries exhausted
- **WHEN** a job with `retry_max: 2` fails on attempt 3 (original + 2 retries)
- **THEN** no further retries are scheduled

#### Scenario: Timed out execution triggers retry
- **WHEN** a job with retries configured times out
- **THEN** a retry is scheduled (timeout is treated as a retryable failure)

### Requirement: No retry on non-transient failures
The system SHALL NOT retry executions that finish with status `Succeeded`, `Cancelled`, or that fail due to assertion failures. Only `Failed` and `TimedOut` statuses trigger retries.

#### Scenario: Successful execution not retried
- **WHEN** an execution succeeds
- **THEN** no retry is scheduled regardless of retry configuration

#### Scenario: Cancelled execution not retried
- **WHEN** an execution is cancelled by the user
- **THEN** no retry is scheduled

#### Scenario: Assertion failure not retried
- **WHEN** an execution succeeds but fails an output assertion (marking it as failed)
- **THEN** no retry is scheduled because assertion failures are logic errors, not transient

### Requirement: Retry delay with exponential backoff
The delay before each retry SHALL be calculated as `retry_delay_secs * retry_backoff^(attempt - 1)`. The delay SHALL be capped at 3600 seconds (1 hour) regardless of the calculated value.

#### Scenario: Fixed delay (backoff = 1.0)
- **WHEN** a job has `retry_delay_secs: 30, retry_backoff: 1.0`
- **THEN** each retry waits 30 seconds

#### Scenario: Exponential backoff
- **WHEN** a job has `retry_delay_secs: 5, retry_backoff: 2.0` and fails three times
- **THEN** retry 1 waits 5s, retry 2 waits 10s, retry 3 waits 20s

#### Scenario: Delay capped at 1 hour
- **WHEN** the calculated delay exceeds 3600 seconds
- **THEN** the actual delay is capped at 3600 seconds

#### Scenario: Zero delay
- **WHEN** a job has `retry_delay_secs: 0`
- **THEN** retries execute immediately with no delay

### Requirement: Retry chain tracking on executions
Each execution record SHALL track its position in a retry chain via `retry_of` (UUID of the original execution) and `attempt_number` (integer starting at 1). These fields SHALL be stored in the `executions` table via migration.

#### Scenario: Original execution has attempt 1
- **WHEN** a job executes for the first time
- **THEN** the execution has `attempt_number: 1` and `retry_of: null`

#### Scenario: First retry has attempt 2
- **WHEN** the original execution (id: X) fails and a retry is scheduled
- **THEN** the retry execution has `attempt_number: 2` and `retry_of: X`

#### Scenario: Second retry links to original
- **WHEN** the first retry (attempt 2) also fails
- **THEN** the second retry has `attempt_number: 3` and `retry_of: X` (same original, not the first retry)

### Requirement: Retry trigger source
The `TriggerSource` enum SHALL include a `Retry` variant that carries the `original_execution_id` (UUID) and `attempt` number. This SHALL be serialized in `triggered_by_json` on execution records.

#### Scenario: Retry execution triggered_by
- **WHEN** a retry execution is created
- **THEN** its `triggered_by` field is `{"type": "retry", "original_execution_id": "...", "attempt": 2}`

#### Scenario: API and UI display
- **WHEN** an execution was triggered by retry
- **THEN** the execution detail shows "Retry (attempt 2/3)" with a link to the original execution

### Requirement: Retry works for agent-dispatched jobs
Retries SHALL work for jobs dispatched to agents (standard and custom). The retry re-dispatches using the same targeting strategy as the original job.

#### Scenario: Agent job retry
- **WHEN** a job targeted at `{"type": "tagged", "tag": "linux"}` fails
- **THEN** the retry dispatches to a tagged "linux" agent (may be a different agent than the original)

#### Scenario: Custom agent job retry
- **WHEN** a custom agent job fails and a retry is scheduled
- **THEN** the retry is enqueued for the same agent via the job queue

### Requirement: Retry config in job create/edit UI
The job creation and edit modals SHALL include retry configuration fields in the Advanced tab: max retries (number input), retry delay (number input in seconds), and backoff multiplier (number input).

#### Scenario: Set retry config in create modal
- **WHEN** a user sets max retries to 3 and delay to 10 seconds in the create modal
- **THEN** the job is created with those retry settings

#### Scenario: Edit retry config
- **WHEN** a user opens the edit modal for a job with retry config
- **THEN** the retry fields are pre-populated with current values
