// Tests for dashboard stat computation logic
// Run with: node web/tests/test_dashboard.js

let passed = 0;
let failed = 0;

function assertEqual(actual, expected, msg) {
    if (JSON.stringify(actual) === JSON.stringify(expected)) { passed++; }
    else { failed++; console.error('FAIL:', msg, '- expected:', JSON.stringify(expected), 'got:', JSON.stringify(actual)); }
}

// Simulate dashboard stat computations
function computeDashStats(jobs, agents) {
    const totalJobs = jobs.length;
    const scheduled = jobs.filter(j => j.status === 'scheduled').length;
    const waiting = jobs.filter(j => (j.depends_on || []).length > 0 && !j.deps_satisfied).length;
    const paused = jobs.filter(j => j.status === 'paused').length;
    const running = jobs.filter(j => j.last_execution && j.last_execution.status === 'running').length;

    let totalExecs = 0, totalSucceeded = 0, totalFailed = 0;
    for (const j of jobs) {
        const c = j.execution_counts || {};
        totalExecs += c.total || 0;
        totalSucceeded += c.succeeded || 0;
        totalFailed += c.failed || 0;
    }

    const onlineAgents = agents.filter(a => a.status === 'online').length;
    const groupSet = new Set(jobs.map(j => j.group || 'Default'));

    return { totalJobs, scheduled, waiting, paused, running, totalExecs, totalSucceeded, totalFailed, onlineAgents, groups: groupSet.size };
}

const jobs = [
    { name: 'j1', status: 'scheduled', depends_on: [], deps_satisfied: true, last_execution: { status: 'running' }, execution_counts: { total: 5, succeeded: 3, failed: 2 }, group: 'A' },
    { name: 'j2', status: 'scheduled', depends_on: [{ job_id: 'x' }], deps_satisfied: false, last_execution: { status: 'succeeded' }, execution_counts: { total: 10, succeeded: 10, failed: 0 }, group: 'A' },
    { name: 'j3', status: 'paused', depends_on: [], deps_satisfied: true, last_execution: null, execution_counts: { total: 0, succeeded: 0, failed: 0 }, group: 'B' },
    { name: 'j4', status: 'scheduled', depends_on: [], deps_satisfied: true, last_execution: { status: 'failed' }, execution_counts: null, group: null },
];

const stats = computeDashStats(jobs, [{ status: 'online' }, { status: 'offline' }]);

assertEqual(stats.totalJobs, 4, 'total jobs');
assertEqual(stats.scheduled, 3, 'scheduled count (includes waiting)');
assertEqual(stats.waiting, 1, 'waiting count');
assertEqual(stats.paused, 1, 'paused count');
assertEqual(stats.running, 1, 'running count');
assertEqual(stats.totalExecs, 15, 'total executions');
assertEqual(stats.totalSucceeded, 13, 'total succeeded');
assertEqual(stats.totalFailed, 2, 'total failed');
assertEqual(stats.onlineAgents, 1, 'online agents');
assertEqual(stats.groups, 3, 'group count (A, B, Default)');

// Edge: null execution_counts
assertEqual(computeDashStats([{ name: 'x', status: 'scheduled', depends_on: [], last_execution: null, execution_counts: null, group: null }], []).totalExecs, 0, 'null execution_counts handled');

// Edge: undefined depends_on
assertEqual(computeDashStats([{ name: 'x', status: 'scheduled', last_execution: null, execution_counts: {}, group: null }], []).waiting, 0, 'undefined depends_on handled');

// Failed jobs for activity card
function getRecentlyFailed(jobs) {
    return jobs.filter(j => j.last_execution && (j.last_execution.status === 'failed' || j.last_execution.status === 'timed_out'));
}

assertEqual(getRecentlyFailed(jobs).length, 1, 'recently failed count');
assertEqual(getRecentlyFailed(jobs)[0].name, 'j4', 'recently failed is j4');
assertEqual(getRecentlyFailed([]).length, 0, 'empty jobs returns empty failed');

console.log('\n' + (passed + failed) + ' tests, ' + passed + ' passed, ' + failed + ' failed');
process.exit(failed > 0 ? 1 : 0);
