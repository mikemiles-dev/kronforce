## ADDED Requirements

### Requirement: Docs page accessible from sidebar
The sidebar SHALL include a "Docs" nav tab with a book icon that navigates to the docs page.

#### Scenario: Clicking Docs tab
- **WHEN** the user clicks the "Docs" tab in the sidebar
- **THEN** the docs page is displayed with the topic list and content area

### Requirement: Docs page has topic navigation
The docs page SHALL have a left-side topic list and a right-side scrollable content area. Clicking a topic SHALL scroll to that section and highlight it in the topic list.

#### Scenario: Navigating between topics
- **WHEN** the user clicks a topic in the sidebar
- **THEN** the content area scrolls to that topic and the topic is visually highlighted

#### Scenario: Topics displayed
- **WHEN** the docs page is shown
- **THEN** topics include: Custom Agents, Scripting, Task Types, API Reference, Cron Expressions

### Requirement: Custom agent guide removed from Settings
The collapsible "Custom Agent Developer Guide" SHALL be removed from the Settings page. Its content is now in the Docs page under "Custom Agents".

#### Scenario: Settings page without agent guide
- **WHEN** the user opens the Settings page
- **THEN** the collapsible custom agent developer guide is no longer present
