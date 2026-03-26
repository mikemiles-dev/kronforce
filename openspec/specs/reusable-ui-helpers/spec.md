### Requirement: View registry manages page visibility
The system SHALL maintain arrays `ALL_VIEWS` and `VIEW_ACTION_BARS` that map page names to DOM element IDs. `showPage()` and `showJobDetail()` SHALL iterate these arrays to toggle visibility instead of hardcoding each element ID.

#### Scenario: Showing a page hides all other views
- **WHEN** `showPage('agents')` is called
- **THEN** all view elements in `ALL_VIEWS` are hidden except `agents-view`, and only `agents-action-bar` is shown from `VIEW_ACTION_BARS`

#### Scenario: Showing job detail hides all views and action bars
- **WHEN** `showJobDetail(id)` is called
- **THEN** all view elements in `ALL_VIEWS` are hidden except `detail-view`, and all action bars in `VIEW_ACTION_BARS` are hidden

#### Scenario: Adding a new page requires only registry updates
- **WHEN** a new page view is added to the application
- **THEN** adding its element ID to `ALL_VIEWS` (and optionally `VIEW_ACTION_BARS`) is sufficient for `showPage()` to manage its visibility

### Requirement: Search filter factory creates page-specific handlers
The system SHALL provide a `createSearchFilter(config)` factory function that accepts `{ inputId, clearBtnId, filterContainerId, debounceMs, onUpdate }` and returns an object with `onSearch()`, `clearSearch()`, and `setStatusFilter(btn, status)` methods.

#### Scenario: Factory produces working search handler
- **WHEN** `createSearchFilter({ inputId: 'agent-search-input', clearBtnId: 'agent-search-clear', onUpdate: renderAgents })` is called
- **THEN** the returned `onSearch()` reads the input value, toggles the clear button visibility, debounces, and calls `renderAgents()`

#### Scenario: Factory produces working clear handler
- **WHEN** the returned `clearSearch()` is called
- **THEN** the input element is cleared, the clear button is hidden, and `onUpdate` is called immediately

#### Scenario: Factory produces working status filter handler
- **WHEN** the returned `setStatusFilter(btn, 'active')` is called
- **THEN** all `.status-btn` elements within `filterContainerId` lose the `active` class, `btn` gains it, and `onUpdate` is called

#### Scenario: All four pages use the factory
- **WHEN** the refactor is complete
- **THEN** jobs, agents, executions, and events pages each use a single `createSearchFilter()` call instead of separate `onSearch`/`clearSearch`/`setStatusFilter` function definitions

### Requirement: Unified info field helper
The system SHALL provide an `infoField(label, value, className)` function that returns `<div class="{className}"><label>{label}</label><div class="value">{value}</div></div>`.

#### Scenario: Replaces execField
- **WHEN** `infoField('Status', 'Running', 'exec-info-item')` is called
- **THEN** it returns the same HTML that `execField('Status', 'Running')` previously returned

#### Scenario: Replaces agentMeta
- **WHEN** `infoField('Version', '1.2', 'agent-meta-item')` is called
- **THEN** it returns the same HTML that `agentMeta('Version', '1.2')` previously returned

### Requirement: Configurable execution table renderer
The system SHALL merge `renderExecTable()` and `renderAllExecsTable()` into a single `renderExecTable(execs, options)` function where `options.showJobColumn` controls whether the Job name column is included.

#### Scenario: Rendering without job column
- **WHEN** `renderExecTable(execs, { showJobColumn: false })` is called
- **THEN** the output matches the previous `renderExecTable()` output (ID, Status, Exit Code, Started, Duration, Agent, Trigger columns)

#### Scenario: Rendering with job column
- **WHEN** `renderExecTable(execs, { showJobColumn: true })` is called
- **THEN** the output matches the previous `renderAllExecsTable()` output (Job column added)

### Requirement: Modal helper functions
The system SHALL provide `openModal(id)` and `closeModal(id)` functions that manage modal overlay visibility. Each modal's overlay element SHALL have a click handler that closes on backdrop click (when `event.target === overlay`).

#### Scenario: Opening a modal
- **WHEN** `openModal('exec-modal')` is called
- **THEN** the element with id `exec-modal` has its `display` style set to `flex` (or equivalent visible state)

#### Scenario: Closing a modal
- **WHEN** `closeModal('exec-modal')` is called
- **THEN** the element with id `exec-modal` has its `display` style set to `none`

#### Scenario: Backdrop click closes modal
- **WHEN** the user clicks the modal overlay (not the modal card content)
- **THEN** the modal closes via `closeModal()`

### Requirement: Empty state helper
The system SHALL provide an `emptyState(message, action)` function that returns an empty state HTML block. The `action` parameter is optional and, when provided, SHALL include a button with the specified label and onclick handler.

#### Scenario: Empty state without action
- **WHEN** `emptyState('No executions found')` is called
- **THEN** it returns `<div class="empty-state"><p>No executions found</p></div>`

#### Scenario: Empty state with action button
- **WHEN** `emptyState('No jobs yet', { label: 'Create your first job', onclick: 'openCreateModal()' })` is called
- **THEN** it returns an empty state div containing the message and a `btn btn-primary` button with the specified label and onclick handler

### Requirement: Form field generator
The system SHALL provide a `formField(config)` function that returns a `<div class="form-group">` HTML string containing a label, an input/select/textarea element based on `config.type`, and an optional hint div.

#### Scenario: Text input field
- **WHEN** `formField({ type: 'text', name: 'job-name', label: 'Name', placeholder: 'my-job' })` is called
- **THEN** it returns a form group div with a label and text input matching the existing create modal markup

#### Scenario: Select field
- **WHEN** `formField({ type: 'select', name: 'task-type', label: 'Type', options: [...] })` is called
- **THEN** it returns a form group div with a label and select element with the specified options

#### Scenario: Field with hint
- **WHEN** `formField({ ..., hint: 'Use kebab-case' })` is called
- **THEN** the form group includes a `<div class="form-hint">Use kebab-case</div>` after the input element
