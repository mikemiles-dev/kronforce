## 1. Database Migration

- [x] 1.1 Create `migrations/0002_audit_log.sql` with the `audit_log` table schema (id, timestamp, actor_key_id, actor_key_name, operation, resource_type, resource_id, details) and indexes on timestamp and operation

## 2. Audit DB Module

- [x] 2.1 Create `src/db/audit.rs` with `Db::record_audit()` method that inserts an audit entry with UUID, current timestamp, actor info from AuthUser, operation, resource_type, resource_id, and optional details
- [x] 2.2 Add `Db::list_audit_log()` method with pagination, optional filters for operation, actor (substring match on actor_key_name), and since (timestamp)
- [x] 2.3 Add `Db::count_audit_log()` method matching the same filters for pagination total
- [x] 2.4 Add `Db::purge_old_audit_log(days)` method that deletes entries older than the given number of days
- [x] 2.5 Register the `audit` module in `src/db/mod.rs`

## 3. Audit Query API

- [x] 3.1 Create `src/api/audit.rs` with `list_audit_log` handler for `GET /api/audit-log` that requires admin role, accepts page/per_page/operation/actor/since query params, and returns paginated results
- [x] 3.2 Register the `/api/audit-log` route in `src/api/mod.rs` under the authenticated router
- [x] 3.3 Register the `audit` module in `src/api/mod.rs`

## 4. Add Audit Recording to API Handlers

- [x] 4.1 Add `record_audit` call to `api/auth.rs` for `key.created` and `key.revoked` operations
- [x] 4.2 Add `record_audit` call to `api/jobs.rs` for `job.created`, `job.updated`, `job.deleted`, and `job.triggered` operations
- [x] 4.3 Add `record_audit` call to `api/scripts.rs` for `script.saved` and `script.deleted` operations
- [x] 4.4 Add `record_audit` call to `api/settings.rs` for `settings.updated` operation
- [x] 4.5 Add `record_audit` call to `api/variables.rs` for `variable.created`, `variable.updated`, and `variable.deleted` operations
- [x] 4.6 Add `record_audit` call to `api/agents.rs` for `agent.deregistered` operation

## 5. Audit Retention

- [x] 5.1 Add `purge_old_audit_log` call to the health monitor loop in `src/bin/controller.rs`, reading `audit_retention_days` from settings (default 90)

## 6. Verify

- [x] 6.1 Run `cargo check` and `cargo test` to verify compilation and all existing tests pass
- [x] 6.2 Run `cargo clippy` to verify no new warnings
