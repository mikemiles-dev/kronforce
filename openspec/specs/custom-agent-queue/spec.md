### Requirement: Queue includes job_id in enqueue and dequeue
The system SHALL store `job_id` in the `job_queue` table when enqueuing a job for a custom agent. The dequeue response SHALL include `job_id` so the agent can reference the originating job.

#### Scenario: Enqueuing a job stores job_id
- **WHEN** a job is dispatched to a custom agent via `enqueue_job()`
- **THEN** the `job_id` from the originating job is stored in the `job_queue` row

#### Scenario: Dequeuing a job returns job_id
- **WHEN** a custom agent polls `GET /api/agent-queue/{agent_id}/next` and receives a job
- **THEN** the JSON response includes a `job_id` field with the originating job's ID

#### Scenario: Database migration adds job_id column
- **WHEN** the controller starts with a database from a previous version
- **THEN** migration version 8 adds a nullable `job_id` column to the `job_queue` table

### Requirement: Stale pending queue items are failed after timeout
The system SHALL periodically check for queue items with status `pending` that have been waiting longer than 5 minutes. These items SHALL be marked as `completed` in the queue, and the corresponding execution SHALL be updated to `failed` status.

#### Scenario: Unclaimed job times out
- **WHEN** a queue item has `status = 'pending'` and `created_at` is older than 5 minutes
- **THEN** the queue item status is set to `completed` and the execution is updated to `failed` with stderr `"queued for custom agent but never claimed (timeout)"`

#### Scenario: Agent polls before timeout
- **WHEN** a queue item has `status = 'pending'` and `created_at` is less than 5 minutes old
- **THEN** the queue item remains in `pending` status and is available for the agent to claim

### Requirement: Stale claimed queue items are failed after timeout
The system SHALL periodically check for queue items with status `claimed` that have not been completed within 10 minutes of being claimed. These items SHALL be marked as `completed` in the queue, and the corresponding execution SHALL be updated to `failed` status.

#### Scenario: Claimed job never reports result
- **WHEN** a queue item has `status = 'claimed'` and `claimed_at` is older than 10 minutes
- **THEN** the queue item status is set to `completed` and the execution is updated to `failed` with stderr `"custom agent claimed job but never reported result (timeout)"`

#### Scenario: Agent reports result before timeout
- **WHEN** a queue item has `status = 'claimed'` and `claimed_at` is less than 10 minutes old
- **THEN** the queue item remains in `claimed` status until the agent reports a result

### Requirement: Cleanup runs in agent health monitor loop
The system SHALL run stale queue cleanup as part of the existing agent health monitor loop that runs every 10 seconds.

#### Scenario: Health monitor performs cleanup
- **WHEN** the agent health monitor loop ticks
- **THEN** it calls both `fail_stale_pending_queue_items` and `fail_stale_claimed_queue_items` after expiring offline agents

### Requirement: UI shows queued badge for custom agent executions
The frontend SHALL display a "queued" badge instead of "pending" for executions that are pending AND assigned to a custom agent.

#### Scenario: Execution pending on custom agent
- **WHEN** an execution has status `pending` and its `agent_id` resolves to an agent with `agent_type = "custom"`
- **THEN** the UI displays a "queued" badge with distinct styling instead of the default "pending" badge

#### Scenario: Execution pending without agent or on standard agent
- **WHEN** an execution has status `pending` and has no `agent_id` or its agent is a standard agent
- **THEN** the UI displays the normal "pending" badge

### Requirement: Executions page has Pending status filter
The executions action bar SHALL include a "Pending" status filter button that filters executions to show only those with `pending` status.

#### Scenario: User filters by pending status
- **WHEN** the user clicks the "Pending" filter button on the executions page
- **THEN** only executions with status `pending` are shown

#### Scenario: Pending filter uses API parameter
- **WHEN** the "Pending" filter is active
- **THEN** the API call includes `?status=pending` query parameter
