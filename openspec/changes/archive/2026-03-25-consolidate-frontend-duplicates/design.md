## Context

The frontend is a single-file vanilla JS/HTML application (`src/dashboard.html`, ~6000 lines) with no framework or module system. All state, DOM manipulation, and rendering happens in global functions within a single `<script>` block. The application has 8 page views (dashboard, jobs, detail, map, agents, executions, scripts, events, settings), each with its own search, filter, table rendering, and state management code — most of which follows identical patterns.

There is no build step or bundler; the file is served directly. This means all helpers must remain as plain JS functions in the same file — no imports or modules.

## Goals / Non-Goals

**Goals:**
- Eliminate duplicated logic by introducing parameterized helper functions
- Reduce the total JS/HTML line count by ~1500 lines
- Make future page additions require less boilerplate
- Preserve all existing behavior exactly — zero visual or functional changes

**Non-Goals:**
- Migrating to a framework (Vue, React, etc.)
- Splitting into multiple files or introducing a build system
- Changing the CSS architecture or design system
- Adding new features or changing existing UI behavior

## Decisions

### 1. Parameterized helpers over class abstractions

**Decision**: Introduce plain functions with config objects rather than JS classes or a component system.

**Rationale**: The codebase is vanilla JS with no OOP patterns. Plain functions (`createSearchHandler(config)`, `renderTable(columns, data)`) are the lowest-friction approach and match existing code style. A class-based system would feel foreign here and risk over-engineering a refactor.

**Alternative considered**: A lightweight component class system — rejected because it introduces patterns the rest of the codebase doesn't use and adds complexity without proportional benefit.

### 2. View registry array for page management

**Decision**: Replace the hardcoded `getElementById` chains in `showPage()` and `showJobDetail()` with a `ALL_VIEWS` and `VIEW_ACTION_BARS` array that maps page names to element IDs.

```js
const ALL_VIEWS = ['dashboard','jobs','detail','map','agents','executions','scripts','events','settings'];
const VIEW_ACTION_BARS = { jobs: 'jobs-action-bar', agents: 'agents-action-bar', executions: 'executions-action-bar', events: 'events-action-bar' };
```

`showPage()` and `showJobDetail()` iterate these arrays instead of listing every element. Adding a new page means adding one entry to each array.

**Rationale**: This is the highest-value, lowest-risk change — it eliminates ~30 lines of repetitive DOM manipulation per call site and makes the view list a single source of truth.

### 3. Factory function for search/filter behavior

**Decision**: Create a `createSearchFilter(config)` factory that returns `{ onSearch, clearSearch, setStatusFilter }` functions, wired to the correct DOM element IDs and state variables.

```js
function createSearchFilter({ inputId, clearBtnId, filterContainerId, stateKey, onUpdate }) {
    // returns { onSearch, clearSearch, setStatusFilter }
}
```

Each page calls the factory once during init and binds the returned functions to its HTML `onclick`/`oninput` handlers.

**Rationale**: The 4 search handlers (`onSearch`, `onAgentSearch`, `onExecSearch`, `onEventSearch`) and their corresponding `clear*` and `setStatusFilter*` functions are structurally identical — only the element IDs and callback differ. A factory eliminates all 12 duplicate functions.

### 4. Unified info field and empty state helpers

**Decision**: Replace `execField()` and `agentMeta()` with a single `infoField(label, value, className)` function. Replace inline empty state HTML with `emptyState(message, action?)`.

**Rationale**: These are trivially identical — `execField` produces `<div class="exec-info-item">` and `agentMeta` produces `<div class="agent-meta-item">`. A single function with a `className` parameter covers both. Same for the 4 empty state patterns.

### 5. Configurable table renderer for executions

**Decision**: Merge `renderExecTable()` and `renderAllExecsTable()` into `renderExecTable(execs, { showJobColumn })`. The only difference is whether the "Job" column is included.

**Rationale**: Both functions produce the same table structure with the same formatters. A boolean flag is simpler than maintaining two 50+ line functions.

### 6. Modal helper function

**Decision**: Create `openModal(id, title, contentHtml)` and `closeModal(id)` helpers that manage overlay visibility and structure. Existing modal IDs and content generation remain — only the open/close/overlay boilerplate is extracted.

**Rationale**: The 3 modals share identical overlay + card + close-on-backdrop-click patterns. The content itself is different enough that it stays as-is; we only extract the structural shell.

### 7. Form field generator

**Decision**: Create `formField(type, name, label, attrs)` that returns the `<div class="form-group"><label>...<input>...</div>` HTML string.

**Rationale**: The create job modal has ~20 form fields following the same pattern. A generator reduces this to ~20 one-liner calls and makes the form structure consistent by construction.

## Risks / Trade-offs

- **Regression risk** → Mitigated by testing each page's search, filter, table, and modal behavior after each consolidation step. Changes are pure refactors with no behavior modification.
- **Readability for contributors unfamiliar with factory patterns** → Mitigated by keeping helpers simple (plain functions, no closures-returning-closures) and adding a brief comment block at the top of the helpers section.
- **HTML event handler wiring becomes indirect** → `onclick="jobSearch.onSearch()"` is slightly less obvious than `onclick="onSearch()"`. Acceptable trade-off for eliminating 12 duplicate functions.
- **Large diff in a single file** → Since everything is in one file, the diff will touch many lines. Mitigated by making changes incrementally (one helper category at a time) so each step can be verified independently.
