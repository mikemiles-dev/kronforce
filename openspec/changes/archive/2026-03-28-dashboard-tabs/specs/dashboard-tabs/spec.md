## ADDED Requirements

### Requirement: Tab bar rendering
The dashboard SHALL render a horizontal tab bar below the stats cards with four tabs: "Overview", "Charts", "Activity", and "Infrastructure". Each tab SHALL be a clickable button. The active tab SHALL be visually distinguished with the accent color.

#### Scenario: Dashboard loads with default tab
- **WHEN** the dashboard page loads
- **THEN** the tab bar is rendered with "Overview" as the active tab and only the Overview panel is visible

#### Scenario: Tab button display
- **WHEN** the tab bar is rendered
- **THEN** all four tab buttons ("Overview", "Charts", "Activity", "Infrastructure") are visible in a horizontal row

### Requirement: Tab switching
Clicking a tab button SHALL hide the currently visible panel and show the selected tab's panel. The tab bar active state SHALL update to reflect the newly selected tab. Tab switching SHALL NOT trigger any API calls or data re-fetching.

#### Scenario: Switch from Overview to Charts
- **WHEN** the user clicks the "Charts" tab
- **THEN** the Overview panel is hidden, the Charts panel is shown, and the "Charts" button is marked active

#### Scenario: Switch back to Overview
- **WHEN** the user clicks the "Overview" tab while on another tab
- **THEN** the other panel is hidden, the Overview panel is shown, and the "Overview" button is marked active

#### Scenario: No network requests on tab switch
- **WHEN** the user switches between any tabs
- **THEN** no API calls are made — all content was rendered on initial dashboard load

### Requirement: Tab content assignment
Each tab SHALL display specific dashboard cards grouped by purpose.

#### Scenario: Overview tab content
- **WHEN** the "Overview" tab is active
- **THEN** the execution timeline and recent executions table are visible

#### Scenario: Charts tab content
- **WHEN** the "Charts" tab is active
- **THEN** the three donut charts (Execution Outcomes, Task Types, Schedule Types) are visible

#### Scenario: Activity tab content
- **WHEN** the "Activity" tab is active
- **THEN** the recent events list is visible

#### Scenario: Infrastructure tab content
- **WHEN** the "Infrastructure" tab is active
- **THEN** the agents summary and dependency map are visible

### Requirement: Stats cards always visible
The stats cards row SHALL remain above the tab bar and be visible regardless of which tab is active. The stats cards SHALL NOT be inside any tab panel.

#### Scenario: Stats visible on Overview tab
- **WHEN** the "Overview" tab is active
- **THEN** the stats cards row is visible above the tab bar

#### Scenario: Stats visible on Charts tab
- **WHEN** the "Charts" tab is active
- **THEN** the stats cards row is visible above the tab bar

### Requirement: Session tab persistence
The active tab SHALL be remembered in a JavaScript variable during the session. When `renderDashboard()` is called again (e.g., navigating away and back), it SHALL restore the previously active tab instead of resetting to Overview.

#### Scenario: Navigate away and back
- **WHEN** the user switches to the "Charts" tab, navigates to the Jobs page, then returns to the dashboard
- **THEN** the "Charts" tab is active (not reset to Overview)

#### Scenario: Page reload resets to default
- **WHEN** the user reloads the browser page
- **THEN** the dashboard opens with the "Overview" tab active (session variable is lost)

### Requirement: Tab bar responsive layout
The tab bar SHALL display tabs horizontally on wide viewports. On narrow viewports (mobile), the tab bar SHALL remain horizontal but allow horizontal scrolling if tabs overflow.

#### Scenario: Wide viewport
- **WHEN** the viewport is wider than 600px
- **THEN** all four tabs are visible in a single row without scrolling

#### Scenario: Narrow viewport
- **WHEN** the viewport is narrower than 600px
- **THEN** the tab bar is horizontally scrollable and does not wrap to multiple lines
