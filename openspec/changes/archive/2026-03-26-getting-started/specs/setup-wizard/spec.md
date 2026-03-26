## ADDED Requirements

### Requirement: Wizard appears on first visit with empty instance
The setup wizard SHALL appear as a full-screen modal overlay when the dashboard loads and both conditions are met: `wizard_completed` setting is not set, and zero jobs exist.

#### Scenario: First visit with empty instance
- **WHEN** a user loads the dashboard for the first time with no jobs and no `wizard_completed` setting
- **THEN** the setup wizard overlay appears on top of the dashboard

#### Scenario: Returning user with jobs
- **WHEN** a user loads the dashboard and jobs already exist
- **THEN** the wizard does not appear

#### Scenario: Wizard previously completed
- **WHEN** the `wizard_completed` setting is set to `"true"`
- **THEN** the wizard does not appear regardless of job count

### Requirement: Wizard has multi-step navigation
The wizard SHALL have a step indicator (numbered dots), content area, and Back/Next/Skip buttons. Steps progress linearly with the ability to skip any step.

#### Scenario: Navigating forward
- **WHEN** the user clicks Next on step 2
- **THEN** step 3 is shown and the step indicator updates

#### Scenario: Skipping a step
- **WHEN** the user clicks Skip on any step
- **THEN** the next step is shown without performing any action

#### Scenario: Going back
- **WHEN** the user clicks Back on step 3
- **THEN** step 2 is shown

#### Scenario: Dismissing the wizard
- **WHEN** the user clicks a close/dismiss button at any point
- **THEN** the wizard closes and `wizard_completed` is set to `"true"`

### Requirement: Step 1 is a welcome screen
The first wizard step SHALL show a brief welcome message introducing Kronforce's key capabilities.

#### Scenario: Welcome step content
- **WHEN** the wizard opens
- **THEN** step 1 shows a welcome title, brief feature highlights (job scheduling, agents, scripting, event triggers), and a "Let's get started" next button

### Requirement: Step 2 offers job template selection
The second step SHALL show clickable template cards for common job types, each pre-filling a job creation form.

#### Scenario: Selecting a health check template
- **WHEN** the user clicks the "Health Check" template card
- **THEN** a pre-filled job form appears within the wizard with HTTP GET task type, a URL input, and cron every 5 minutes

#### Scenario: Selecting a cron task template
- **WHEN** the user clicks the "Cron Task" template card
- **THEN** a pre-filled job form appears with shell task type and cron schedule

#### Scenario: Creating a job from the wizard
- **WHEN** the user fills in the template form and clicks Create
- **THEN** the job is created via the API and a success message is shown in the wizard

#### Scenario: Choosing custom
- **WHEN** the user clicks "Custom" or "Create from scratch"
- **THEN** the wizard closes and the full create job modal opens

### Requirement: Step 3 shows agent connection instructions
The third step SHALL show how to connect a standard agent and link to custom agent documentation.

#### Scenario: Agent setup instructions
- **WHEN** the wizard shows step 3
- **THEN** the step displays a copy-paste terminal command for starting a standard agent and a link to the Docs page for custom agents

### Requirement: Step 4 offers quick notification setup
The fourth step SHALL offer a streamlined notification configuration.

#### Scenario: Quick notification config
- **WHEN** the wizard shows step 4
- **THEN** the step shows email recipient input and a toggle for agent offline alerts, with a Save button that persists to settings

### Requirement: Step 5 shows completion summary
The final step SHALL summarize what was set up and provide next-step links.

#### Scenario: Completion with job created
- **WHEN** the user reaches step 5 after creating a job
- **THEN** the summary shows the job name, links to the Jobs page, Docs page, and a "Finish" button that closes the wizard

#### Scenario: Finish sets wizard_completed
- **WHEN** the user clicks Finish
- **THEN** `wizard_completed` is set to `"true"` in settings and the wizard closes
