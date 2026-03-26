## Context

The dashboard is a single-file HTML app. Pages are `<section>` elements toggled by `showPage()`. Empty states currently exist for jobs, agents, and scripts but are minimal (just text + a button). The settings table stores key-value pairs for runtime config.

## Goals / Non-Goals

**Goals:**
- First-visit wizard that appears once and guides through initial setup
- Rich empty states on every page with contextual help and quick actions
- Zero backend changes — purely frontend + one setting key

**Non-Goals:**
- Interactive tutorials or tooltips that follow you around
- Video walkthroughs
- Sample data seeding (the wizard creates real jobs)

## Decisions

### 1. Wizard as a modal overlay with step navigation

**Decision**: The wizard is a full-screen modal overlay (similar to the existing modal pattern) with a step indicator at the top, content area in the middle, and Back/Next/Skip buttons at the bottom. Steps are numbered dots showing progress.

Each step is a function that renders its content into the wizard body. Navigation is handled by `wizardNext()`, `wizardBack()`, `wizardSkip()`.

**Rationale**: Reuses the existing modal overlay pattern. Step functions keep each step self-contained. No new routing needed.

### 2. Wizard detection via settings + job count

**Decision**: On dashboard load, check:
1. `GET /api/settings` — if `wizard_completed` is set, don't show
2. If not set, `GET /api/jobs` — if zero jobs exist, show the wizard

This avoids showing the wizard to existing users who upgrade. Only truly empty instances see it.

After completion or dismissal, `PUT /api/settings { wizard_completed: "true" }` persists the flag.

**Rationale**: Simple heuristic. Existing instances with jobs skip the wizard. New instances get the full experience.

### 3. Job templates for quick-create

**Decision**: The "Create your first job" step offers 3-4 pre-filled templates as clickable cards:

- **Health Check** — HTTP GET to a URL, cron every 5 minutes
- **Cron Task** — Shell command on a schedule
- **Event Watcher** — Event-triggered job that reacts to failures
- **Custom** — Opens the full create modal

Clicking a template pre-fills the create modal with sensible defaults. The user can edit before saving.

**Rationale**: Templates reduce decision fatigue. Users see concrete examples of what Kronforce can do. The "Custom" option is always available for advanced users.

### 4. Empty states as HTML functions

**Decision**: Create a `renderEmptyState(config)` function that generates rich empty state HTML from a config object:

```javascript
renderEmptyState({
    icon: '📋',
    title: 'No jobs yet',
    description: 'Jobs are automated tasks that run on a schedule, on events, or on demand.',
    actions: [
        { label: 'Create a health check', onclick: "openTemplateJob('health-check')" },
        { label: 'Create a cron job', onclick: "openTemplateJob('cron')" },
        { label: 'Create from scratch', onclick: 'openCreateModal()' },
    ],
    hint: 'Or connect an agent to run jobs on remote machines →',
})
```

This replaces the current inline `emptyState()` helper with a richer version.

**Rationale**: Consistent component across all pages. The config pattern makes each page's empty state declarative and easy to update.

### 5. Wizard steps don't navigate away

**Decision**: The wizard overlay stays on top of the dashboard. Steps that create things (jobs, settings) use the existing API and show success inline in the wizard — they don't close the wizard and open the create modal.

Exception: The "Custom" template option closes the wizard and opens the create modal directly.

**Rationale**: Keeping the user in the wizard maintains flow. Opening modals within modals is confusing.

## Risks / Trade-offs

- **Wizard detection is a heuristic** → Existing users who delete all jobs would see the wizard. Acceptable — they can dismiss it and it won't show again.
- **Templates may not match user's use case** → The "Custom" option and skip button are always available. Templates are suggestions, not requirements.
- **Dashboard load makes extra API calls for wizard detection** → Two lightweight GET calls on load. Negligible latency.
