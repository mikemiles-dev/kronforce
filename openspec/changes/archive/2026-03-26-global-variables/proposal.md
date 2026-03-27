## Why

Jobs often need to reference shared values — API endpoints, credentials, thresholds, environment names — that change across deployments or over time. Today these values are hardcoded in each job's task definition, making bulk updates tedious and error-prone. Global variables provide a single place to manage these values, with the ability for jobs to update them dynamically via output extraction.

## What Changes

- Add a **global variables** store (name/value pairs) persisted in the database
- Add a **Variables page** in the dashboard UI for CRUD management of variables
- Support **`{{VAR_NAME}}` substitution** in task fields (command, URL, query, message body, etc.) before execution — both local and remote
- Extend **output extraction** so extracted values can optionally write back to a global variable
- Add a **REST API** for variable CRUD (`/api/variables`)
- Pass resolved variables to **remote agents** so substitution works for dispatched jobs

## Capabilities

### New Capabilities
- `global-variables`: Storage, API, UI management, and substitution of global key-value variables in task definitions
- `variable-extraction`: Extending output extraction rules to write extracted values back into global variables

### Modified Capabilities
- `output-extraction`: Add optional `write_to_variable` field on extraction rules so extracted values can update global variables

## Impact

- **Database**: New `variables` table, new migration
- **Models**: New `Variable` struct, updated `ExtractionRule` with optional `write_to_variable` field
- **API**: New `/api/variables` endpoints (GET, POST, PUT, DELETE)
- **Executor**: Variable substitution step before task execution in both local and dispatch paths
- **Dashboard**: New Variables page, updated extraction rule UI to support write-back
- **Agent protocol**: `JobDispatchRequest` extended with resolved variable values
