## ADDED Requirements

### Requirement: Chart data aggregation API
The system SHALL provide a `GET /api/stats/charts` endpoint that returns aggregated data for dashboard charts. The endpoint SHALL require authentication (same as other API endpoints). The response SHALL contain three objects: `execution_outcomes`, `task_types`, and `schedule_types`.

#### Scenario: Successful chart data fetch
- **WHEN** an authenticated user requests `GET /api/stats/charts`
- **THEN** the system returns a 200 response with JSON containing `execution_outcomes` (map of status string to count), `task_types` (map of task type name to job count), and `schedule_types` (map of schedule kind to job count)

#### Scenario: Execution outcomes aggregation
- **WHEN** the system computes `execution_outcomes`
- **THEN** it SHALL return counts for each execution status across all jobs, with keys `succeeded`, `failed`, `timed_out`, `cancelled`, `running`, and `pending`

#### Scenario: Task type aggregation
- **WHEN** the system computes `task_types`
- **THEN** it SHALL count the number of jobs using each task type variant (Shell, Http, Script, Sql, Ftp, FilePush, Kafka, Rabbitmq, Mqtt, Redis, Custom) and return only types with count > 0

#### Scenario: Schedule type aggregation
- **WHEN** the system computes `schedule_types`
- **THEN** it SHALL count the number of jobs using each schedule kind (Cron, OnDemand, OneShot, Event) and return only kinds with count > 0

#### Scenario: No jobs exist
- **WHEN** the system has no jobs
- **THEN** the endpoint SHALL return all three objects with empty maps

#### Scenario: Unauthenticated request
- **WHEN** an unauthenticated request is made to `GET /api/stats/charts` and API keys are configured
- **THEN** the system SHALL return 401 Unauthorized

### Requirement: Donut chart SVG rendering
The system SHALL render donut charts as inline SVG using `<circle>` elements with `stroke-dasharray` for segment sizing. Each chart SHALL display a centered label showing the total count. Each chart SHALL include a legend listing segment labels with their colors and counts.

#### Scenario: Chart with multiple segments
- **WHEN** chart data contains multiple categories with non-zero values
- **THEN** the system SHALL render one SVG circle stroke segment per category, sized proportionally to each category's share of the total, with distinct colors per segment

#### Scenario: Chart with more than 6 categories
- **WHEN** chart data contains more than 6 categories
- **THEN** the system SHALL display the top 5 categories by count and group the remainder into an "Other" segment

#### Scenario: Chart with no data
- **WHEN** chart data is empty or all values are zero
- **THEN** the system SHALL display a "No data" message centered in the chart area instead of an empty donut

#### Scenario: Single category
- **WHEN** chart data contains only one category
- **THEN** the system SHALL render a full donut ring in that category's color

### Requirement: Chart color theming
Chart segment colors SHALL use CSS custom properties to support light and dark themes. The primary statuses (succeeded, failed) SHALL use `--success` and `--danger` respectively. Additional segments SHALL use `--warning`, `--info`, `--accent`, and a muted neutral color.

#### Scenario: Dark theme active
- **WHEN** the user has dark theme enabled
- **THEN** chart segment colors, legend text, and center label SHALL adapt via CSS custom properties without JavaScript changes

#### Scenario: Light theme active
- **WHEN** the user has light theme enabled
- **THEN** chart segment colors, legend text, and center label SHALL adapt via CSS custom properties without JavaScript changes

### Requirement: Dashboard chart layout
The dashboard SHALL display three chart cards in a grid row positioned between the stats bar and the execution timeline. Each card SHALL have a header title and contain one donut chart with its legend.

#### Scenario: Dashboard renders with charts
- **WHEN** the dashboard page loads
- **THEN** the system SHALL fetch chart data from `GET /api/stats/charts` and render three chart cards: "Execution Outcomes", "Task Types", and "Schedule Types"

#### Scenario: Charts fetch failure
- **WHEN** the chart data API request fails
- **THEN** the chart cards SHALL display gracefully without crashing the dashboard, showing the "No data" empty state

#### Scenario: Responsive layout
- **WHEN** the viewport width is narrow (mobile or small screen)
- **THEN** the chart cards SHALL stack vertically instead of displaying in a three-column row
