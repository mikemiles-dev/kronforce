## Why

The `src/` directory has 12 loose `.rs` files alongside module directories, and `dashboard.html` is an 8,137-line monolith containing all CSS, HTML, and JavaScript. This makes the codebase hard to navigate, slows down UI development, and makes it difficult to work on individual features without scrolling through thousands of unrelated lines.

## What Changes

- **Consolidate loose Rust files** into logical module directories under `src/` (e.g., `src/scheduler/`, `src/models/`, etc.)
- **Move `dashboard.html`** out of `src/` into a `web/` directory at the project root
- **Split `dashboard.html`** into separate files:
  - `web/index.html` — HTML structure only (~1,450 lines)
  - `web/css/style.css` — All CSS (~2,188 lines)
  - `web/js/` — JavaScript split into multiple modules by feature area (~4,490 lines total across ~8-10 files)
- **Update the build** to concatenate/inline web assets at compile time via `include_str!` or a build script, preserving the single-binary deployment model

## Capabilities

### New Capabilities
- `web-asset-structure`: How web assets (HTML, CSS, JS) are organized on disk, concatenated at build time, and served as a single-page app
- `rust-module-structure`: How Rust source files are organized into module directories

### Modified Capabilities
- `module-structure`: Update existing module structure spec to reflect new directory layout

## Impact

- **All Rust source files** — many files move into subdirectories, `mod.rs` and `lib.rs` updated
- **Build system** — new `build.rs` or macro to concatenate web assets into a single string for `include_str!`
- **`api/mod.rs`** — `include_str!` path changes to reference concatenated output
- **No runtime behavior changes** — the served HTML is identical, just built from multiple source files
- **No API changes** — all endpoints remain the same
