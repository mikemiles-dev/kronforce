// Launches a kronforce controller against a temp DB + scripts dir, waits for
// /api/health, then writes connection details to e2e/.runtime.json so tests can
// pick up the bootstrap admin key.

const { spawn } = require('child_process');
const fs = require('fs');
const os = require('os');
const path = require('path');

const PORT = process.env.KRONFORCE_E2E_PORT || '18080';
const ADMIN_KEY = 'kf_e2e_admin_key_for_testing_only_xxxxxx';
const RUNTIME_FILE = path.join(__dirname, '.runtime.json');
const REPO_ROOT = path.resolve(__dirname, '..');

async function waitForReady(url, timeoutMs = 30_000) {
    const deadline = Date.now() + timeoutMs;
    while (Date.now() < deadline) {
        try {
            const res = await fetch(url + '/api/health');
            if (res.ok) return;
        } catch (_) { /* not up yet */ }
        await new Promise(r => setTimeout(r, 200));
    }
    throw new Error(`controller did not become ready at ${url} within ${timeoutMs}ms`);
}

module.exports = async () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'kronforce-e2e-'));
    const dbPath = path.join(tmpDir, 'kronforce.db');
    const scriptsDir = path.join(tmpDir, 'scripts');
    fs.mkdirSync(scriptsDir, { recursive: true });

    const binary = path.join(REPO_ROOT, 'target', 'debug', 'kronforce');
    if (!fs.existsSync(binary)) {
        throw new Error(
            `kronforce binary not found at ${binary}. ` +
            `Run \`cargo build --bin kronforce\` from the repo root first.`
        );
    }

    const logPath = path.join(tmpDir, 'kronforce.log');
    const logFd = fs.openSync(logPath, 'a');
    const child = spawn(binary, [], {
        env: {
            ...process.env,
            KRONFORCE_BIND: `127.0.0.1:${PORT}`,
            KRONFORCE_DB: dbPath,
            KRONFORCE_SCRIPTS_DIR: scriptsDir,
            KRONFORCE_BOOTSTRAP_ADMIN_KEY: ADMIN_KEY,
            RUST_LOG: process.env.RUST_LOG || 'kronforce=warn',
        },
        stdio: ['ignore', logFd, logFd],
        detached: false,
    });

    child.on('error', (err) => {
        console.error('failed to spawn kronforce:', err);
    });

    fs.writeFileSync(
        RUNTIME_FILE,
        JSON.stringify({
            pid: child.pid,
            port: PORT,
            adminKey: ADMIN_KEY,
            tmpDir,
            scriptsDir,
            logPath,
        }, null, 2)
    );

    try {
        await waitForReady(`http://127.0.0.1:${PORT}`);
    } catch (e) {
        const log = fs.existsSync(logPath) ? fs.readFileSync(logPath, 'utf8') : '(no log)';
        try { process.kill(child.pid, 'SIGTERM'); } catch (_) {}
        throw new Error(`${e.message}\n--- controller log ---\n${log}`);
    }
};
