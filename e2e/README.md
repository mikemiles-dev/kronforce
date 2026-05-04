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

`tests/regressions.spec.js` covers the four bugs we shipped fixes for in
0.2.1-alpha:

- Browser back button walks through in-app navigation (`pushState`
  vs `replaceState`).
- Settings → Agents card click does not blank the page (dead
  `showPage('agents')` handler).
- Script editor opens with the actual code visible in the textarea
  (overlay-pattern regression).
- Highlight overlay sits above the textarea so colored spans show
  (z-index regression).
- Event rows expose a clickable `output` chip when an `execution_id` is
  attached.

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
