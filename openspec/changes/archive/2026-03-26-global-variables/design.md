## Context

Kronforce jobs currently have hardcoded values in their task definitions (commands, URLs, query strings, message bodies). There is no mechanism to share or reuse values across jobs, and no way for a job's output to feed into another job's input beyond event-driven triggering. The output extraction system already extracts values from stdout into `ExecutionRecord.extracted` but these are read-only historical records with no write-back capability.

## Goals / Non-Goals

**Goals:**
- Provide a persistent key-value store for global variables accessible to all jobs
- Substitute `{{VAR_NAME}}` placeholders in task fields before execution
- Allow output extraction rules to write extracted values back to global variables
- Expose full CRUD via REST API and dashboard UI
- Work consistently for both local execution and remote agent dispatch

**Non-Goals:**
- Per-job or per-agent scoped variables (all variables are global)
- Variable versioning or audit history
- Secret management (variables are stored as plain text, same as task definitions)
- Variable typing or validation (all values are strings)
- Nested variable references (`{{{{VAR}}}}` expanding to another variable name)

## Decisions

### 1. Substitution syntax: `{{VAR_NAME}}`

Use double-curly-brace syntax for variable references in task fields.

**Why not `${VAR_NAME}`:** Conflicts with shell variable expansion in command strings — a shell task containing `${HOME}` should expand via the shell, not Kronforce. Double-curlies are unambiguous and visually distinct.

**Why not Handlebars-style `{{var.name}}`:** No need for nested namespaces. Simple flat key-value is sufficient. Variable names restricted to `[A-Za-z0-9_]`.

### 2. Substitution happens controller-side before dispatch

Variable resolution occurs in the controller before the task is sent to an agent or executed locally. The agent receives a fully-resolved task with no `{{...}}` placeholders remaining.

**Why not agent-side:** Agents would need access to the variable store, adding complexity to the agent protocol and requiring agents to authenticate for variable reads. Controller-side is simpler — agents stay stateless.

**Trade-off:** Variables reflect their value at dispatch time, not execution time. For jobs with long queue waits this could matter, but this matches user expectations (the job runs with the values it was dispatched with).

### 3. Database: dedicated `variables` table

```sql
CREATE TABLE variables (
    name TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

Simple key-value with timestamp. No UUID — the name is the natural key. This avoids join complexity and makes the API intuitive (`GET /api/variables/MY_VAR`).

### 4. Substitution applies to all string fields in TaskType

Rather than maintaining a whitelist of substitutable fields, serialize the entire `TaskType` to JSON, perform string replacement on the JSON string, then deserialize back. This automatically covers all current and future task type fields (command, url, query, body, broker, topic, etc.).

**Risk:** A variable value containing JSON-special characters (quotes, backslashes) could break deserialization. **Mitigation:** When substituting into JSON, escape the variable value for JSON string context (escape `"`, `\`, and control characters). Unresolved `{{VAR}}` references where the variable doesn't exist are left as-is with a warning logged.

### 5. Extraction write-back via optional `write_to_variable` field

Add an optional `write_to_variable: String` field to `ExtractionRule`. After extraction runs and produces a value, if `write_to_variable` is set, upsert that value into the `variables` table.

**Why on ExtractionRule:** Reuses the existing extraction pipeline. No new task type needed — any job with output extraction can update variables. The extraction result is still stored on the `ExecutionRecord` as before; the write-back is a side effect.

### 6. API design

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/variables` | List all variables |
| `GET` | `/api/variables/{name}` | Get single variable |
| `POST` | `/api/variables` | Create variable (`{name, value}`) |
| `PUT` | `/api/variables/{name}` | Update variable value |
| `DELETE` | `/api/variables/{name}` | Delete variable |

Protected by existing API key auth. Requires `admin` or `operator` role for writes, `viewer` for reads.

### 7. UI: dedicated Variables page

Add a new "Variables" nav item in the dashboard sidebar/header. The page shows a table of all variables with inline edit, add, and delete. Keep it simple — similar pattern to the existing Settings page but with a dynamic list rather than fixed fields.

The job modal's extraction rule editor gets a new optional "Write to variable" text input per extraction rule.

## Risks / Trade-offs

- **Circular updates:** Job A extracts a value into `VAR_X`, Job B uses `{{VAR_X}}` and writes to `VAR_Y`, Job A uses `{{VAR_Y}}`. This is valid and useful (pipeline-style), but users could create infinite loops if jobs trigger each other via events. → No mitigation needed beyond existing event loop protections.
- **Race conditions on write-back:** Two jobs extracting to the same variable concurrently — last write wins. → Acceptable for v1. The `updated_at` timestamp provides visibility into when a value last changed.
- **JSON substitution edge cases:** Variable values with special characters could produce invalid JSON after substitution. → Mitigate by JSON-escaping values during substitution.
- **Missing variables:** If a `{{VAR}}` reference doesn't match any defined variable, the placeholder is left as-is. → Log a warning so users can diagnose. Don't fail the job — a hardcoded fallback in the command may be intentional.
