## 1. Database & Models

- [x] 1.1 Add migration v13: create `variables` table (name TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT NOT NULL)
- [x] 1.2 Add `Variable` struct to `models.rs` (name, value, updated_at)
- [x] 1.3 Add optional `write_to_variable: Option<String>` field to `ExtractionRule` in `models.rs`
- [x] 1.4 Add DB CRUD functions for variables: list, get, create, update, delete, upsert (in `db/variables.rs` or existing db module)

## 2. Variables API

- [x] 2.1 Add `GET /api/variables` endpoint — list all variables (viewer+ role)
- [x] 2.2 Add `GET /api/variables/{name}` endpoint — get single variable (viewer+ role)
- [x] 2.3 Add `POST /api/variables` endpoint — create variable with name validation (`[A-Za-z0-9_]`) (operator+ role)
- [x] 2.4 Add `PUT /api/variables/{name}` endpoint — update variable value (operator+ role)
- [x] 2.5 Add `DELETE /api/variables/{name}` endpoint (operator+ role)
- [x] 2.6 Register variable routes in the API router

## 3. Variable Substitution

- [x] 3.1 Add `substitute_variables` function: serialize TaskType to JSON, replace `{{VAR_NAME}}` patterns with JSON-escaped values, deserialize back
- [x] 3.2 Integrate substitution into local executor path (before `run_task` in `executor/local.rs`)
- [x] 3.3 Integrate substitution into remote dispatch path (before building `JobDispatchRequest` in `executor/dispatch.rs`)
- [x] 3.4 Integrate substitution into custom agent queue path (before `enqueue_job`)
- [x] 3.5 Log warnings for unresolved `{{VAR}}` placeholders

## 4. Extraction Write-Back

- [x] 4.1 Update `run_extractions` or post-extraction logic to check `write_to_variable` on each rule
- [x] 4.2 After extraction produces a value with `write_to_variable` set, upsert the value into the `variables` table
- [x] 4.3 Handle the case where extraction pattern doesn't match (no variable write)

## 5. Dashboard — Variables Page

- [x] 5.1 Add "Variables" nav item to the dashboard header/sidebar
- [x] 5.2 Add Variables page with table displaying name, value, updated_at columns
- [x] 5.3 Add "Add Variable" button and form (name + value inputs with name validation)
- [x] 5.4 Add inline edit for variable values
- [x] 5.5 Add delete button with confirmation
- [x] 5.6 Add empty state message when no variables exist

## 6. Dashboard — Extraction Rule Editor Update

- [x] 6.1 Add optional "Write to variable" text input to each extraction rule row in the job modal
- [x] 6.2 Validate `write_to_variable` name matches `[A-Za-z0-9_]` pattern
- [x] 6.3 Include `write_to_variable` when saving/loading extraction rules

## 7. Tests

- [x] 7.1 Test variable CRUD DB operations (create, read, update, delete, upsert, name validation)
- [x] 7.2 Test `substitute_variables` function (single var, multiple vars, missing var, special chars, no vars)
- [x] 7.3 Test extraction write-back (match with write-back, no match, create vs update)
- [x] 7.4 Test variable API endpoints (CRUD, role permissions, validation errors)
