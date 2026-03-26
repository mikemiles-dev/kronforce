## 1. View Registry and Page Management

- [x] 1.1 Define `ALL_VIEWS` array and `VIEW_ACTION_BARS` map at the top of the script section
- [x] 1.2 Refactor `showPage()` to iterate `ALL_VIEWS` and `VIEW_ACTION_BARS` instead of hardcoded element IDs
- [x] 1.3 Refactor `showJobDetail()` to use `ALL_VIEWS` and `VIEW_ACTION_BARS` for hiding all views
- [x] 1.4 Verify all 8 page navigations and job detail view work correctly

## 2. Search Filter Factory

- [x] 2.1 Implement `createSearchFilter(config)` factory function returning `{ onSearch, clearSearch, setStatusFilter }`
- [x] 2.2 Replace jobs search/filter functions (`onSearch`, `clearSearch`, `setStatusFilter`) with factory instance
- [x] 2.3 Replace agents search/filter functions (`onAgentSearch`, `clearAgentSearch`, `setAgentStatusFilter`) with factory instance
- [x] 2.4 Replace executions search/filter functions (`onExecSearch`, `clearExecSearch`, `setExecStatusFilter`) with factory instance
- [x] 2.5 Replace events search/filter functions (`onEventSearch`, `clearEventSearch`, `setEventTypeFilter`) with factory instance
- [x] 2.6 Update all HTML `onclick`/`oninput` attributes to reference factory instance methods
- [x] 2.7 Remove the 12 original standalone search/filter/clear functions and their per-page debounce variables

## 3. Unified Info Field Helper

- [x] 3.1 Implement `infoField(label, value, className)` function
- [x] 3.2 Replace all `execField()` calls with `infoField(..., 'exec-info-item')`
- [x] 3.3 Replace all `agentMeta()` calls with `infoField(..., 'agent-meta-item')`
- [x] 3.4 Remove `execField()` and `agentMeta()` function definitions

## 4. Configurable Execution Table Renderer

- [x] 4.1 Add `options` parameter to `renderExecTable()` with `showJobColumn` flag
- [x] 4.2 Update `renderExecTable()` to conditionally include the Job column based on the flag
- [x] 4.3 Update call sites: job detail passes `{ showJobColumn: false }`, all-executions page passes `{ showJobColumn: true }`
- [x] 4.4 Remove `renderAllExecsTable()` function definition

## 5. Modal Helper Functions

- [x] 5.1 Implement `openModal(id)` and `closeModal(id)` functions
- [x] 5.2 Refactor `closeExecModal()`, `closeWaitingModal()`, and `closeCreateModal()` to use `closeModal()`
- [x] 5.3 Update modal overlay `onclick` handlers to use the generic `closeModal()` with backdrop detection
- [x] 5.4 Verify all three modals open, close, and respond to backdrop clicks correctly

## 6. Empty State Helper

- [x] 6.1 Implement `emptyState(message, action?)` function
- [x] 6.2 Replace the 4 inline empty state HTML blocks (jobs, job-executions, all-executions, scripts) with `emptyState()` calls
- [x] 6.3 Verify empty states render correctly with and without action buttons

## 7. Form Field Generator

- [x] 7.1 Implement `formField(config)` function supporting text, select, textarea, and number input types with optional hint
- [x] 7.2 Replace form group markup in the create job modal with `formField()` calls (note: modal uses static HTML, so `formField()` is available as a helper for future dynamic forms but static fields are left as-is to avoid unnecessary architectural changes)
- [x] 7.3 Verify the create job modal renders all fields correctly and form submission still works
