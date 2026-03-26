## Context

The dashboard is a single-file HTML application embedded in the Rust binary via `include_str!`. All pages are `<section>` elements toggled by `showPage()`. Adding a docs page follows the same pattern.

## Goals / Non-Goals

**Goals:**
- Docs page accessible from sidebar with a book icon
- Topic list on the left, content on the right (or stacked on mobile)
- Topics: Custom Agents, Scripting, Task Types, API Reference, Cron Expressions
- Content is static HTML embedded in the page (no markdown rendering needed)
- Remove the collapsible guide from Settings

**Non-Goals:**
- External markdown file loading or build step
- Search within docs
- Editable/user-contributed docs

## Decisions

### 1. Two-pane layout within the docs section

**Decision**: The docs view uses a flex layout with a narrow topic sidebar (200px) and a scrollable content area. Each topic is a `<div>` with an ID, all stacked in the content area. Clicking a topic scrolls to it and highlights the active topic in the sidebar.

**Rationale**: Simpler than hiding/showing separate divs per topic. All content is always in the DOM, and the topic sidebar acts as a table of contents with scroll-to behavior.

### 2. Content as static HTML in dashboard.html

**Decision**: Doc content is written directly as HTML in the `docs-view` section. No markdown parsing or external files.

**Rationale**: Matches the rest of the dashboard. Adding a markdown parser would increase complexity for minimal benefit. HTML gives full control over formatting.

### 3. Register docs view in ALL_VIEWS

**Decision**: Add `'docs'` to the `ALL_VIEWS` array so `showPage()` handles it automatically.

**Rationale**: Uses the existing view registry from the earlier refactor.

## Risks / Trade-offs

- **Large HTML file gets larger** → Acceptable. The dashboard is already 6000+ lines and docs content compresses well in HTTP responses.
- **Content maintenance** → Updating docs requires editing dashboard.html and rebuilding. Acceptable for a self-contained tool.
