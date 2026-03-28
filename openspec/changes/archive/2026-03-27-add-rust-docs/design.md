## Context

The Kronforce codebase has good documentation on core types (`db/models.rs`, `error.rs`, `config.rs`) but most public functions and methods — particularly API handlers, database operations, agent protocol types, and executor internals — have no doc comments. This change adds `///` and `//!` doc comments across the codebase without modifying any behavior.

## Goals / Non-Goals

**Goals:**
- Add doc comments to all undocumented public structs, enums, traits, functions, and methods
- Add module-level `//!` docs where missing (e.g., `lib.rs`, `agent/mod.rs`)
- Follow existing documentation style already established in well-documented files like `error.rs` and `db/models.rs`

**Non-Goals:**
- Refactoring code to be more "documentable"
- Adding inline `//` implementation comments
- Generating external documentation sites or mdBook
- Documenting private/internal helper functions
- Adding `#[doc(hidden)]` or rustdoc attributes
- Changing any runtime behavior

## Decisions

### 1. Document by module, not by item type

Work through each module directory (`api/`, `db/`, `agent/`, `executor/`, `scheduler/`) as a unit rather than doing "all structs first, then all functions." This keeps related context together and reduces back-and-forth.

**Why:** Each module has its own domain vocabulary (e.g., agent protocol vs. API handlers). Documenting by module lets the writer stay in context and produce more consistent docs within each domain.

### 2. Follow existing style from `db/models.rs` and `error.rs`

Use the same concise, declarative style already present in the codebase:
- One-line summary for simple items
- Multi-line with blank line separator for complex items
- Document enum variants inline

**Why:** Consistency with existing docs. No need to establish a new convention — one already exists in the best-documented files.

### 3. Prioritize public API surface over internal helpers

Focus on `pub` and `pub(crate)` items. Skip private functions and closures unless they are particularly complex.

**Why:** Public items are the contract other modules depend on. Private helpers are implementation details that change frequently.

### 4. Order of work: protocol → db → api → executor → scheduler

Start with `agent/protocol.rs` (small, self-contained, defines the wire format), then `db/` methods (foundational layer), then `api/` handlers (largest gap), then `executor/` and `scheduler/`.

**Why:** Protocol types define the system's communication contract and are referenced everywhere. Database methods are the next foundational layer. API handlers are the largest body of undocumented code but depend on understanding db and protocol first.

## Risks / Trade-offs

- **Stale docs risk** → Mitigated by keeping doc comments close to the code they describe and using concise, behavioral descriptions rather than implementation details that drift.
- **Large diff size** → Mitigated by splitting work into per-module tasks. Each module can be reviewed independently.
- **Subjective quality** → Mitigated by following the existing style from `db/models.rs` as the reference standard.
