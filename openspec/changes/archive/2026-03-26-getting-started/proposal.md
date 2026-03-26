## Why

New users landing on Kronforce for the first time see an empty dashboard with no guidance on what to do next. Each page has minimal empty states that don't help users understand the product's capabilities or how to get started. A guided wizard on first visit plus rich empty states throughout the app would dramatically reduce time-to-value and help users discover features they might not find on their own.

## What Changes

- **First-visit setup wizard**: A multi-step overlay that appears on first login (detected by zero jobs + no `wizard_completed` setting). Walks the user through:
  1. **Welcome** — brief intro to Kronforce capabilities
  2. **Create your first job** — guided form with pre-filled examples for common patterns (health check, cron task, shell script)
  3. **Connect an agent** (optional) — copy-paste command for standard agent, link to custom agent docs
  4. **Set up notifications** (optional) — quick email/SMS config
  5. **Done** — summary of what was set up, links to docs and next steps
  - Each step has a Skip button. Completing or skipping all steps sets `wizard_completed` setting so it doesn't show again.

- **Rich empty states**: Enhanced empty state components on each page with:
  - **Jobs**: Quick-create buttons for common job templates (health check, cron job, file deploy, event trigger)
  - **Agents**: Copy-paste agent start command, link to custom agent docs, visual showing agent architecture
  - **Executions**: Explanation + link to create and trigger a job
  - **Events**: Explanation of what generates events
  - **Scripts**: Example script templates to click and create
  - **Map**: Explanation of dependency visualization with a prompt to create dependent jobs

## Capabilities

### New Capabilities
- `setup-wizard`: Multi-step first-visit onboarding wizard with guided job creation, agent setup, and notification config
- `rich-empty-states`: Enhanced empty state components with contextual help, quick-create actions, and feature discovery

### Modified Capabilities

## Impact

- **Frontend**: New wizard overlay component, enhanced empty state HTML/JS on each page
- **Settings**: New `wizard_completed` setting to track wizard state
- **No backend changes**: Wizard uses existing APIs (create job, settings). Empty states are purely frontend.
- **No database changes**: Uses existing settings table
