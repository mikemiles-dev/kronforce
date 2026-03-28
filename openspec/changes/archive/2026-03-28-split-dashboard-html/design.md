## Context

The dashboard UI is a vanilla JS SPA served as a single bundled HTML file. `build.rs` already reads separate CSS and JS files and injects them into `web/index.html` at compile time. The HTML itself (1,461 lines) is still monolithic, containing all 10 views, 4 action bars, 6 modals, login screen, and inline docs in one file. The CSS (2,186 lines) and JS (6,684 lines across 10 files) are already well-separated.

## Goals / Non-Goals

**Goals:**
- Split `web/index.html` into partial HTML files in `web/partials/`
- Extend `build.rs` to read partials and inject them at placeholder markers
- Keep the final bundled `dashboard.html` output functionally identical
- Make each view independently editable without touching other views

**Non-Goals:**
- Changing the CSS architecture (already in a separate file)
- Splitting the JS further (already in 10 files)
- Adding a JS framework or build tool (webpack, vite, etc.)
- Making the dashboard serve assets separately (it remains a single embedded HTML)
- Splitting CSS into per-component files

## Decisions

### 1. Use `<!-- INCLUDE:filename -->` placeholder pattern

Add `<!-- INCLUDE:partials/sidebar.html -->` style markers in `index.html`. The `build.rs` will find these markers and replace each with the contents of the referenced file, relative to `web/`.

**Why:** This follows the same pattern already used for `<!-- INJECT:CSS -->` and `<!-- INJECT:JS -->`. It's simple, grep-able, and doesn't require a template engine. The placeholder is self-documenting — you can see which partial goes where.

**Alternative considered:** Using a generic template engine crate — rejected because build.rs should stay simple and dependency-free. The current string replacement approach works well.

### 2. Group partials by type: views, action-bars, and top-level components

```
web/partials/
  login.html              # Login screen
  sidebar.html            # Navigation sidebar
  action-bars.html        # All 4 action bars (jobs, agents, events, executions)
  modals.html             # All 6 modal overlays
  views/
    dashboard.html
    jobs.html              # Jobs list + job detail
    executions.html
    agents.html
    scripts.html
    events.html
    variables.html
    settings.html
    docs.html
    map.html
```

**Why:** Mirrors the existing JS file organization (one JS file per view). The `views/` subdirectory groups the 10 page sections. Action bars are kept together since they're small and structurally similar. Modals stay in one file since they share the overlay pattern.

### 3. Process includes recursively (single level)

`build.rs` will scan the HTML for `<!-- INCLUDE:path -->` markers and replace each with the file contents. Only one level deep — partials cannot include other partials.

**Why:** Single-level inclusion is sufficient for this structure. Recursive includes add complexity and risk of circular references with no benefit here.

### 4. Add `cargo::rerun-if-changed` for all partial files

`build.rs` will watch the `web/partials/` directory so changes to any partial trigger a rebuild.

**Why:** Without this, editing a partial wouldn't trigger recompilation, leading to stale dashboard content.

## Risks / Trade-offs

- **Build output not byte-identical** → The include markers are replaced with file contents. Whitespace may differ slightly at injection points. This is acceptable since the HTML is not minified.
- **More files to manage** → 14 new partial files vs. 1 large file. Net improvement in navigability outweighs the file count.
- **Partial boundaries may split mid-element** → Mitigated by splitting at clean section boundaries (between `<section>` tags, between modals, etc.).
