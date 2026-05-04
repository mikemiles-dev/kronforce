# Kronforce E2E Tests

End-to-end UI tests using [Playwright](https://playwright.dev/). They start a
real `kronforce` controller against a temp DB + scripts dir on port 18080 and
drive a headless Chromium browser against it. Each test file picks up a
shared admin key written by `global-setup.js` so seeding can be done over the
REST API.

## First-time setup

```sh
cd e2e
npm install
npx playwright install chromium       # downloads the browser binary
```

You also need a built kronforce binary at `target/debug/kronforce`:

```sh
cargo build --bin kronforce           # from the repo root
```

## Running

```sh
cd e2e
npm test                  # headless run
npm run test:headed       # see the browser
npm run test:ui           # Playwright UI mode (best for debugging)
npm run report            # open the HTML report from the last run
```

## What's covered today

| Spec | Focus |
|---|---|
| `regressions.spec.js` | Bugs we already fixed in 0.2.1-alpha — back button, blank Settings → Agents click, script editor visible code, highlight z-index, event chip affordance. Treat as the canary. |
| `smoke.spec.js` | Every top-level page renders, sub-tabs swap content, sidebar links route. Catches broad nav regressions. |
| `jobs.spec.js` | API-create job → visible on Monitor; search filter; click-into detail; trigger creates execution; delete removes from list. |
| `scripts.spec.js` | UI flow: + New Script saves, click loads code, type-switch swaps templates, delete removes from list. |
| `variables.spec.js` | UI add/list/secret-masking/search. Delete via API (UI uses `confirm()`). |
| `settings.spec.js` | All tabs swap content, retention persists across reload, notification toggle round-trips. |
| `agents-list.spec.js` | API-register agent → visible under Settings → Agents with hostname/address/tags. |

## Adding tests

- Drop a new `*.spec.js` file under `tests/`.
- Use `helpers.js` to seed data: `api('POST', '/api/jobs', {...})` runs as
  admin against the shared controller. Use `writeLocalScript()` for scripts —
  they're served straight from disk by `ScriptStore`.
- Use `openApp(page, '#/some/route')` to land the SPA already authenticated
  (the helper injects the admin key into `localStorage` before app JS runs).

Tests share a single controller (`workers: 1` in `playwright.config.js`)
because the SQLite DB and scripts dir are global state. Either keep tests
data-isolated by using unique names, or clean up after yourself in
`afterEach`.
