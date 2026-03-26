## Why

The main frontend (`src/dashboard.html`) is a 6000+ line single-file application with extensive code duplication across search/filter logic, table rendering, modals, info display helpers, and page management. This duplication makes changes error-prone (fixing a bug in one search handler means fixing it in four places) and inflates the codebase by an estimated 1500+ lines of near-identical code.

## What Changes

- **Consolidate search/filter functions**: Unify `onSearch`, `onAgentSearch`, `onExecSearch`, `onEventSearch` (and their corresponding `clear*` and `setStatusFilter` variants) into parameterized helpers
- **Consolidate table rendering**: Merge `renderExecTable()` and `renderAllExecsTable()` into a single configurable table renderer
- **Unify info display helpers**: Merge `execField()` and `agentMeta()` (functionally identical, different CSS class) into one `renderInfoItem()` function
- **Consolidate modal structures**: Extract repeated modal overlay/card/close patterns into a reusable `openModal(title, content, actions)` helper
- **Consolidate page view management**: Replace repeated `getElementById(...).style.display = 'none'` blocks in `showJobDetail()` and `showPage()` with a `hideAllViews()`/`showView()` helper
- **Consolidate empty states**: Unify 4 near-identical empty state patterns into an `emptyState(message, action?)` helper
- **Consolidate form field markup**: Extract repeated `<div class="form-group">` patterns in the create modal into a `formField()` generator

## Capabilities

### New Capabilities
- `reusable-ui-helpers`: Shared helper functions for search bars, filters, modals, tables, info fields, empty states, form fields, and page view management

### Modified Capabilities

## Impact

- **Code**: `src/dashboard.html` — all changes are within this single file
- **Risk**: Low — these are pure refactors with no behavioral changes; all existing UI and interactions remain identical
- **Estimated reduction**: ~1500 lines of duplicate code consolidated into parameterized helpers
