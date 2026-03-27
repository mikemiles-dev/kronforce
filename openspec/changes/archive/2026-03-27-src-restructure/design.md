## Context

The `src/` directory currently has 12 loose `.rs` files alongside 4 module directories (`api/`, `db/`, `executor/`, `agent/`). The dashboard UI is a single 8,137-line `dashboard.html` file containing ~2,200 lines of CSS, ~1,450 lines of HTML, and ~4,500 lines of JavaScript with ~215 functions. It's embedded into the binary via `include_str!("../dashboard.html")` in `api/mod.rs`, enabling zero-dependency single-binary deployment.

## Goals / Non-Goals

**Goals:**
- Organize loose Rust files into module directories for discoverability
- Break the monolithic dashboard into separate HTML, CSS, and JS files
- Split JavaScript into feature-based modules (jobs, agents, executions, etc.)
- Preserve single-binary deployment — no runtime static file serving
- Make it easy to work on one UI feature without scrolling past thousands of unrelated lines

**Non-Goals:**
- Introducing a JavaScript build tool (webpack, esbuild, vite, etc.)
- Adding TypeScript or a frontend framework
- Changing any runtime behavior or API
- Restructuring the `api/`, `db/`, `executor/`, or `agent/` module directories (they're already well-organized)

## Decisions

### 1. Web asset directory: `web/` at project root

Place all web source files in a top-level `web/` directory:

```
web/
  index.html          # HTML structure only
  css/
    style.css         # All CSS
  js/
    app.js            # Bootstrap, routing, API client, utilities
    dashboard.js      # Dashboard stats/charts page
    jobs.js           # Job CRUD, list, detail, form
    executions.js     # Execution list, detail, output viewer
    agents.js         # Agent list, task type editor
    scripts.js        # Script editor
    events.js         # Event log
    variables.js      # Variables page
    settings.js       # Settings tabs, auth, keys, notifications
    modals.js         # Modal/form utilities, cron builder, output rules editor
```

**Why `web/` at root, not `src/web/`:** These aren't Rust source files. Keeping them at the root makes clear they're web assets, similar to how `scripts/` and `docs/` live at root. Also avoids cluttering `src/` further.

**Why this JS split:** Each file maps to a nav page (jobs, agents, executions, etc.), matching how developers think about the UI. `app.js` holds shared infrastructure (API client, routing, toast, badges). `modals.js` holds cross-cutting form utilities (cron builder, output rules editor, task form, modal open/close).

### 2. Build-time concatenation via `build.rs`

Add a `build.rs` that concatenates all web assets into a single HTML string at compile time:

```
index.html + <style>{style.css}</style> + <script>{app.js + all page JS}</script>
```

The output is written to `OUT_DIR/dashboard.html`. Then `api/mod.rs` changes to:

```rust
const DASHBOARD_HTML: &str = include_str!(concat!(env!("OUT_DIR"), "/dashboard.html"));
```

**Why `build.rs` over a macro:** `build.rs` is the standard Rust approach for code generation. It runs before compilation, produces a file in `OUT_DIR`, and `include_str!` picks it up. No proc-macro crate needed. Also, `build.rs` can list all web files so cargo knows to rebuild when any of them change.

**Why not keep `include_str!` with multiple calls:** Can't concatenate multiple `include_str!` calls into one HTML document — need to inject `<style>` and `<script>` tags around the content.

### 3. Rust module consolidation: group by domain

Move loose files into directories only where there's a clear grouping:

| Current | New | Rationale |
|---------|-----|-----------|
| `scheduler.rs` | `src/scheduler.rs` | Keep as-is — standalone, no sub-modules needed |
| `cron_parser.rs` | `src/cron_parser.rs` | Keep as-is — used only by scheduler |
| `models.rs` | `src/models.rs` | Keep as-is — widely imported, splitting adds friction |
| `config.rs` | `src/config.rs` | Keep as-is — small, standalone |
| `error.rs` | `src/error.rs` | Keep as-is — small, standalone |
| `dag.rs` | `src/dag.rs` | Keep as-is — small, standalone |
| `protocol.rs` | `src/protocol.rs` | Keep as-is — small, standalone |
| `notifications.rs` | `src/notifications.rs` | Keep as-is — standalone |
| `output_rules.rs` | `src/output_rules.rs` | Keep as-is — standalone |
| `scripts.rs` | `src/scripts.rs` | Keep as-is — standalone |
| `dashboard.html` | `web/index.html` + split | **Move and split** |

**Why keep most Rust files in place:** These files are small (50–400 lines), self-contained, and don't have sub-modules. Moving a 93-line `config.rs` into `src/config/mod.rs` adds a directory and an `mod.rs` file for no benefit. The real mess is `dashboard.html`, not the Rust layout. The Rust files are already reasonably organized — the 4 multi-file modules (`api/`, `db/`, `executor/`, `agent/`) are properly in directories.

### 4. JavaScript global scope and shared state

Since we're not using a bundler, all JS files share the global scope (concatenated into one `<script>` tag). Each file uses an IIFE-like pattern or just declares its functions at top level (matching the current style). Shared state (like `allJobs`, `allAgents`, `currentPage`) stays as globals in `app.js`.

**Load order matters:** `app.js` must come first (defines `api()`, `esc()`, `toast()`, `badge()`, etc.), then page modules in any order, then the initialization code at the bottom of `app.js` calls `checkAuth()` and sets up routing.

To handle this, `build.rs` concatenates files in a defined order: `app.js` first, then alphabetical page files, ensuring the shared utilities are available.

### 5. HTML uses placeholder comments for injection

`index.html` contains:

```html
<!-- INJECT:CSS -->
<!-- INJECT:JS -->
```

`build.rs` replaces these with the inlined content. This keeps `index.html` viewable in a browser during development (albeit unstyled) and makes the injection points explicit.

## Risks / Trade-offs

- **Large diff** — Touching every line of an 8,137-line file means git history for the old file is lost. → Accept this; `git log --follow` won't work well anyway for a split. The archive in openspec preserves the "before" state.
- **JS load order bugs** — If a page file references a function from `app.js` that hasn't loaded yet. → Mitigate with `build.rs` enforcing `app.js` first, and testing in browser after the split.
- **Build complexity** — Adding `build.rs` is a new moving part. → It's simple string concatenation with no dependencies. If it breaks, error messages are clear.
- **Dev workflow change** — Previously edit one file, now edit one of ~12. → This is the entire point. Each file is focused and navigable.
