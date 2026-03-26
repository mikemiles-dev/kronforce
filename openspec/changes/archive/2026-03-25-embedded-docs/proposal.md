## Why

The README has grown to 800+ lines covering custom agents, scripting, task types, cron expressions, dependencies, and authentication. This content should be browsable within the dashboard itself, and the README should be trimmed to a concise overview with pointers to the in-app docs. The collapsible agent setup guide currently lives in the Settings page which is the wrong location.

## What Changes

- **New "Docs" page in the dashboard**: Add a "Docs" nav tab in the sidebar. The page shows a left-side topic list and right-side content area with rendered documentation.
- **Doc topics**: Custom Agents, Scripting (Rhai), Task Types, API Reference, Cron Expressions — content pulled from README sections.
- **Move agent guide**: Remove the collapsible agent developer guide from the Settings section and integrate it into the Docs page under "Custom Agents".
- **Trim README**: Keep README as a concise project overview with Quick Start, Architecture diagram, and links to the in-app docs for details.
- **Docs embedded in binary**: Like the dashboard, docs content is part of `dashboard.html` (no external files to serve).

## Capabilities

### New Capabilities
- `embedded-docs-page`: In-app documentation page with topic navigation and rendered content

### Modified Capabilities

## Impact

- **Frontend**: New `docs-view` section, sidebar nav tab, topic navigation, content rendering
- **Settings page**: Remove the custom agent developer guide collapsible
- **README**: Trimmed significantly with pointers to in-app docs
