## 1. Rich Empty State Component

- [x] 1.1 Create `renderRichEmptyState(config)` function that accepts `{ icon, title, description, actions: [{label, onclick, primary}], hint }` and returns styled HTML
- [x] 1.2 Add CSS for rich empty state (centered card, icon, title, description, action button row, hint)

## 2. Rich Empty States on Each Page

- [x] 2.1 Replace jobs page empty state with rich version: "No jobs yet" + template buttons (Health Check, Cron Task, Event Watcher, Create from scratch)
- [x] 2.2 Replace agents page empty state with rich version: "No agents registered" + copy-paste command + link to Docs
- [x] 2.3 Replace all-executions page empty state with rich version: "No executions yet" + prompt to create and trigger a job
- [x] 2.4 Update map page empty state with rich version: explain dependency graph + prompt to add dependencies
- [x] 2.5 Update events page empty rendering with rich version: "No events yet" + explanation of what generates events

## 3. Job Templates

- [x] 3.1 Create `openTemplateJob(template)` function that opens the create modal with pre-filled values based on template name
- [x] 3.2 Define templates: health-check (HTTP GET, cron 5min), cron-task (shell, cron hourly), event-watcher (event schedule, execution.completed, severity error)
- [x] 3.3 Wire template buttons from empty states and wizard to `openTemplateJob()`

## 4. Setup Wizard — Structure

- [x] 4.1 Add wizard overlay HTML: full-screen modal with step dots, content area, Back/Next/Skip/Close buttons
- [x] 4.2 Add CSS for wizard (overlay, step indicator dots, step content, navigation buttons, template cards)
- [x] 4.3 Add `showWizard()`, `closeWizard()`, `wizardNext()`, `wizardBack()`, `wizardSkip()` navigation functions
- [x] 4.4 Add wizard state tracking: `wizardStep`, `wizardData` (stores what was created)

## 5. Setup Wizard — Steps

- [x] 5.1 Step 1 (Welcome): Title, feature highlights list (scheduling, agents, scripting, events, notifications), "Let's get started" button
- [x] 5.2 Step 2 (Create Job): Template cards (Health Check, Cron Task, Event Watcher, Custom). Clicking a card shows an inline form. Create button calls API.
- [x] 5.3 Step 3 (Connect Agent): Copy-paste command for standard agent, link to custom agent docs, "Skip if running locally" note
- [x] 5.4 Step 4 (Notifications): Email recipient input + agent offline toggle. Save button persists to settings. Skip option.
- [x] 5.5 Step 5 (Done): Summary of what was set up, links to Jobs page and Docs page, "Finish" button

## 6. Setup Wizard — Detection and Persistence

- [x] 6.1 On dashboard load (`init()`), check settings for `wizard_completed` and job count. If not completed and zero jobs, call `showWizard()`
- [x] 6.2 On wizard close/finish, `PUT /api/settings { wizard_completed: "true" }`
