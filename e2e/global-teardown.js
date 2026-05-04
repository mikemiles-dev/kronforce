// Stops the kronforce controller spawned by global-setup.js and cleans up
// the temp dir.

const fs = require('fs');
const path = require('path');

const RUNTIME_FILE = path.join(__dirname, '.runtime.json');

module.exports = async () => {
    if (!fs.existsSync(RUNTIME_FILE)) return;
    const runtime = JSON.parse(fs.readFileSync(RUNTIME_FILE, 'utf8'));
    if (runtime.pid) {
        try { process.kill(runtime.pid, 'SIGTERM'); } catch (_) { /* already gone */ }
        // Give it a moment to flush, then SIGKILL if still alive.
        await new Promise(r => setTimeout(r, 300));
        try { process.kill(runtime.pid, 'SIGKILL'); } catch (_) { /* expected */ }
    }
    if (runtime.tmpDir && fs.existsSync(runtime.tmpDir)) {
        fs.rmSync(runtime.tmpDir, { recursive: true, force: true });
    }
    fs.unlinkSync(RUNTIME_FILE);
};
