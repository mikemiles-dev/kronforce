## ADDED Requirements

### Requirement: Variables are stored as key-value pairs
The system SHALL persist global variables in a `variables` database table with columns `name` (TEXT PRIMARY KEY), `value` (TEXT NOT NULL), and `updated_at` (TEXT NOT NULL). Variable names SHALL be restricted to alphanumeric characters and underscores (`[A-Za-z0-9_]`).

#### Scenario: Creating a variable
- **WHEN** a variable is created with name `API_HOST` and value `https://api.example.com`
- **THEN** a row is inserted into the `variables` table with the name, value, and current timestamp

#### Scenario: Variable name validation
- **WHEN** a variable is created with name `my-var` (contains hyphen)
- **THEN** the system rejects the request with a validation error

#### Scenario: Duplicate variable name
- **WHEN** a variable is created with a name that already exists
- **THEN** the system rejects the request with a conflict error

### Requirement: Variables CRUD API
The system SHALL expose REST endpoints for managing variables at `/api/variables`. Creating, updating, and deleting variables SHALL require `admin` or `operator` role. Listing and reading variables SHALL require at least `viewer` role.

#### Scenario: List all variables
- **WHEN** a GET request is made to `/api/variables`
- **THEN** the response contains an array of all variables with name, value, and updated_at fields

#### Scenario: Get a single variable
- **WHEN** a GET request is made to `/api/variables/API_HOST`
- **THEN** the response contains the variable's name, value, and updated_at

#### Scenario: Get a non-existent variable
- **WHEN** a GET request is made to `/api/variables/MISSING`
- **THEN** the response is 404 Not Found

#### Scenario: Create a variable
- **WHEN** a POST request is made to `/api/variables` with body `{"name": "API_HOST", "value": "https://api.example.com"}`
- **THEN** the variable is created and the response contains the created variable

#### Scenario: Update a variable
- **WHEN** a PUT request is made to `/api/variables/API_HOST` with body `{"value": "https://new-api.example.com"}`
- **THEN** the variable's value and updated_at are updated

#### Scenario: Delete a variable
- **WHEN** a DELETE request is made to `/api/variables/API_HOST`
- **THEN** the variable is removed from the database

#### Scenario: Unauthorized access
- **WHEN** a POST/PUT/DELETE request is made with a `viewer` role API key
- **THEN** the request is rejected with 403 Forbidden

### Requirement: Variable substitution in task fields
Before executing a job (locally or dispatching to an agent), the system SHALL substitute `{{VAR_NAME}}` placeholders in all task type string fields with the corresponding variable value. Substitution SHALL be performed controller-side by serializing the task to JSON, replacing all `{{...}}` patterns with JSON-escaped variable values, and deserializing back.

#### Scenario: Shell command with variable substitution
- **WHEN** a job has a shell task with command `curl {{API_HOST}}/status` and variable `API_HOST` is `https://api.example.com`
- **THEN** the executed command is `curl https://api.example.com/status`

#### Scenario: HTTP task with variable in URL
- **WHEN** a job has an HTTP task with url `{{API_HOST}}/health` and variable `API_HOST` is `https://api.example.com`
- **THEN** the request is made to `https://api.example.com/health`

#### Scenario: Multiple variables in one field
- **WHEN** a task field contains `{{HOST}}:{{PORT}}` and variables `HOST=localhost`, `PORT=8080`
- **THEN** the field resolves to `localhost:8080`

#### Scenario: Undefined variable reference
- **WHEN** a task field contains `{{MISSING_VAR}}` and no variable named `MISSING_VAR` exists
- **THEN** the placeholder `{{MISSING_VAR}}` is left as-is and a warning is logged

#### Scenario: Variable value with special JSON characters
- **WHEN** a variable value contains quotes or backslashes (e.g., `he said "hello"`)
- **THEN** the value is JSON-escaped during substitution so the task JSON remains valid

#### Scenario: Remote agent dispatch with variables
- **WHEN** a job targeting a remote agent contains `{{VAR}}` placeholders
- **THEN** the controller resolves all variables before dispatching the task to the agent

### Requirement: Variables management page in dashboard
The dashboard SHALL include a "Variables" page accessible from the main navigation. The page SHALL display all variables in a table with columns for name, value, and last updated timestamp, with controls to add, edit, and delete variables.

#### Scenario: Viewing the variables page
- **WHEN** the user navigates to the Variables page
- **THEN** a table displays all global variables sorted by name

#### Scenario: Adding a variable from the UI
- **WHEN** the user clicks "Add Variable" and enters name `THRESHOLD` and value `100`
- **THEN** the variable is created via the API and appears in the table

#### Scenario: Editing a variable inline
- **WHEN** the user edits the value of an existing variable and saves
- **THEN** the variable is updated via the API and the table reflects the new value

#### Scenario: Deleting a variable
- **WHEN** the user clicks delete on a variable and confirms
- **THEN** the variable is removed via the API and disappears from the table

#### Scenario: Empty state
- **WHEN** no variables exist
- **THEN** the page displays a message indicating no variables have been created with a prompt to add one
