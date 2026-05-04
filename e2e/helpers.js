// Shared helpers for Playwright tests. Reads runtime info written by
// global-setup.js (admin key, port) and exposes:
//   - api(): authenticated fetch helper for seeding test data via REST
//   - openApp(page): navigates to the controller with the admin key already
//     installed in localStorage so the SPA renders without hitting the login
//     screen
//   - writeLocalScript(): drops a file directly into the script store dir
//     (faster than going through the API for fixtures)

const fs = require('fs');
const path = require('path');

const RUNTIME_FILE = path.join(__dirname, '.runtime.json');

function runtime() {
    if (!fs.existsSync(RUNTIME_FILE)) {
        throw new Error('e2e/.runtime.json missing — global-setup did not run');
    }
    return JSON.parse(fs.readFileSync(RUNTIME_FILE, 'utf8'));
}

function baseUrl() {
    return `http://127.0.0.1:${runtime().port}`;
}

async function api(method, path, body) {
    const { adminKey } = runtime();
    const res = await fetch(baseUrl() + path, {
        method,
        headers: {
            'Authorization': `Bearer ${adminKey}`,
            ...(body !== undefined ? { 'Content-Type': 'application/json' } : {}),
        },
        body: body !== undefined ? JSON.stringify(body) : undefined,
    });
    const text = await res.text();
    if (!res.ok) {
        throw new Error(`${method} ${path} -> ${res.status}: ${text}`);
    }
    return text ? JSON.parse(text) : null;
}

async function openApp(page, hashRoute = '#/dashboard') {
    const { adminKey } = runtime();
    // Inject the API key + suppress the first-time tour into localStorage
    // before any app JS runs. The tour pops up an overlay that intercepts
    // clicks, which fails most interaction tests.
    await page.addInitScript((key) => {
        try {
            localStorage.setItem('kronforce-api-key', key);
            localStorage.setItem('kf-tour-done', '1');
        } catch (_) {}
    }, adminKey);
    await page.goto(baseUrl() + '/' + hashRoute);
}

function writeLocalScript(name, code) {
    const { scriptsDir } = runtime();
    const filePath = path.join(scriptsDir, name);
    fs.writeFileSync(filePath, code);
    return filePath;
}

function deleteLocalScript(name) {
    const { scriptsDir } = runtime();
    const filePath = path.join(scriptsDir, name);
    if (fs.existsSync(filePath)) fs.unlinkSync(filePath);
}

module.exports = { api, openApp, baseUrl, writeLocalScript, deleteLocalScript, runtime };
