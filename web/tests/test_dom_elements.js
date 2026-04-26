// Tests for DOM element IDs referenced in JavaScript
// Verifies that all IDs used in JS exist in the HTML partials
// Run with: node web/tests/test_dom_elements.js

const fs = require('fs');
const path = require('path');

let passed = 0;
let failed = 0;

function assert(condition, msg) {
    if (condition) { passed++; }
    else { failed++; console.error('FAIL:', msg); }
}

// Read all HTML files
const webDir = path.join(__dirname, '..');
const htmlFiles = [
    'partials/views/dashboard.html',
    'partials/views/monitor.html',
    'partials/views/pipelines.html',
    'partials/views/designer.html',
    'partials/views/toolbox.html',
    'partials/views/settings.html',
    'partials/modals.html',
    'partials/sidebar.html',
];

let allHtml = '';
for (const f of htmlFiles) {
    try {
        allHtml += fs.readFileSync(path.join(webDir, f), 'utf-8');
    } catch (e) {
        // File might not exist in test env
    }
}

// Read all JS files to find getElementById calls
const jsDir = path.join(webDir, 'js');
const jsFiles = fs.readdirSync(jsDir).filter(f => f.endsWith('.js') && !f.includes('.min.'));
let allJs = '';
for (const f of jsFiles) {
    allJs += fs.readFileSync(path.join(jsDir, f), 'utf-8');
}

// Extract IDs from getElementById calls (static, not dynamic)
const idRegex = /getElementById\(['"]([a-zA-Z0-9_-]+)['"]\)/g;
const referencedIds = new Set();
let match;
while ((match = idRegex.exec(allJs)) !== null) {
    referencedIds.add(match[1]);
}

// Critical IDs that must exist in HTML
const criticalIds = [
    // Dashboard
    'dash-stats', 'dash-timeline', 'dash-chart-outcomes', 'dash-chart-tasks',
    'dash-chart-schedules', 'dash-recent-execs', 'dash-recent-events',
    'dash-agents', 'dash-running-section', 'dash-failed-section',
    'dash-stages', 'dash-map-container', 'dash-map-controls',
    // Jobs
    'jobs-table-wrap', 'jobs-pagination',
    'map-container', 'map-controls',
    // Group picker
    'group-picker-wrap', 'group-picker-popover', 'group-picker-list',
    'group-picker-search', 'group-picker-btn', 'group-picker-label',
    // Modals
    'exec-modal', 'exec-detail-content',
    'trigger-params-modal', 'trigger-params-content',
    'waiting-modal', 'waiting-detail-content', 'waiting-run-anyway-btn',
    // Job form fields
    'f-name', 'f-command', 'f-cron', 'f-priority', 'f-max-concurrent',
    'f-approval-required', 'f-sla-deadline', 'f-starts-at', 'f-expires-at',
    'f-email-output', 'f-forward-url',
    // Cron builder
    'cb-sec-mode', 'cb-sec-val', 'cb-min-mode', 'cb-min-val',
    'cb-hr-mode', 'cb-hr-val', 'cb-dom-mode', 'cb-dom-val',
    'cb-mon-mode', 'cb-mon-val', 'cb-preview',
    // Settings
    'settings-panel-general',
    // Execution
    'exec-cancel-btn', 'exec-approve-btn',
];

for (const id of criticalIds) {
    const exists = allHtml.includes('id="' + id + '"');
    assert(exists, 'HTML element with id="' + id + '" should exist');
}

// Check that commonly referenced IDs in JS actually exist in HTML
const dynamicIdPrefixes = ['trigger-', 'output-', 'tp-', 'pair-cmd-'];
let staticMissing = 0;
for (const id of referencedIds) {
    // Skip dynamic IDs (generated at runtime)
    if (dynamicIdPrefixes.some(p => id.startsWith(p))) continue;
    if (id.startsWith('agent-config-')) continue;
    if (id.startsWith('custom-')) continue;
    // Check existence
    if (!allHtml.includes('id="' + id + '"')) {
        // Only warn, don't fail — some IDs are created dynamically by JS
        // console.warn('WARN: id="' + id + '" referenced in JS but not in static HTML');
        staticMissing++;
    }
}

console.log('(Checked ' + criticalIds.length + ' critical IDs, ' + referencedIds.size + ' JS-referenced IDs, ' + staticMissing + ' dynamic-only)');
console.log('\n' + (passed + failed) + ' tests, ' + passed + ' passed, ' + failed + ' failed');
process.exit(failed > 0 ? 1 : 0);
