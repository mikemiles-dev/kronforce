// Tests for job filter logic
// Run with: node web/tests/test_job_filters.js

let passed = 0;
let failed = 0;

function assertEqual(actual, expected, msg) {
    if (JSON.stringify(actual) === JSON.stringify(expected)) { passed++; }
    else { failed++; console.error('FAIL:', msg, '- expected:', JSON.stringify(expected), 'got:', JSON.stringify(actual)); }
}

// Simulate the filter logic from applyJobFilters in jobs.js
function applyFilter(jobs, statusFilter, searchTerm, groupFilter) {
    let filtered = jobs;
    if (statusFilter === 'blocked') {
        filtered = filtered.filter(j => j.depends_on.length > 0 && !j.deps_satisfied);
    } else if (statusFilter === 'running') {
        filtered = filtered.filter(j => j.last_execution && j.last_execution.status === 'running');
    } else if (statusFilter === 'failed') {
        filtered = filtered.filter(j => (j.execution_counts && j.execution_counts.failed > 0) || (j.last_execution && (j.last_execution.status === 'failed' || j.last_execution.status === 'timed_out')));
    } else if (statusFilter === 'scheduled') {
        filtered = filtered.filter(j => j.status === 'scheduled');
    } else if (statusFilter === 'paused') {
        filtered = filtered.filter(j => j.status === 'paused');
    } else if (statusFilter === 'unscheduled') {
        filtered = filtered.filter(j => j.status === 'unscheduled' && !(j.depends_on.length > 0 && !j.deps_satisfied));
    }
    if (searchTerm) {
        filtered = filtered.filter(j => j.name.toLowerCase().includes(searchTerm) || (j.description && j.description.toLowerCase().includes(searchTerm)));
    }
    if (groupFilter) {
        filtered = filtered.filter(j => (j.group || 'Default') === groupFilter);
    }
    return filtered;
}

// --- Test data ---
const jobs = [
    { name: 'deploy-prod', status: 'scheduled', group: 'Deploys', depends_on: [], deps_satisfied: true,
      last_execution: { status: 'succeeded' }, execution_counts: { total: 10, succeeded: 8, failed: 2 }, description: 'Production deployment' },
    { name: 'health-check', status: 'scheduled', group: 'Monitoring', depends_on: [], deps_satisfied: true,
      last_execution: { status: 'running' }, execution_counts: { total: 100, succeeded: 99, failed: 1 } },
    { name: 'etl-extract', status: 'scheduled', group: 'ETL', depends_on: [], deps_satisfied: true,
      last_execution: { status: 'failed' }, execution_counts: { total: 5, succeeded: 3, failed: 2 } },
    { name: 'etl-transform', status: 'scheduled', group: 'ETL', depends_on: [{ job_id: '1' }], deps_satisfied: false,
      last_execution: { status: 'succeeded' }, execution_counts: { total: 3, succeeded: 3, failed: 0 } },
    { name: 'backup', status: 'paused', group: 'Maintenance', depends_on: [], deps_satisfied: true,
      last_execution: { status: 'succeeded' }, execution_counts: { total: 50, succeeded: 50, failed: 0 } },
    { name: 'old-job', status: 'unscheduled', group: null, depends_on: [], deps_satisfied: true,
      last_execution: null, execution_counts: { total: 0, succeeded: 0, failed: 0 } },
    { name: 'timed-out-job', status: 'scheduled', group: 'ETL', depends_on: [], deps_satisfied: true,
      last_execution: { status: 'timed_out' }, execution_counts: { total: 1, succeeded: 0, failed: 1 } },
];

// --- No filter (All) ---
assertEqual(applyFilter(jobs, '', '', '').length, 7, 'all filter returns all jobs');

// --- Running filter ---
const running = applyFilter(jobs, 'running', '', '');
assertEqual(running.length, 1, 'running filter count');
assertEqual(running[0].name, 'health-check', 'running filter matches health-check');

// --- Failed filter ---
const failedJobs = applyFilter(jobs, 'failed', '', '');
assertEqual(failedJobs.length, 4, 'failed filter count (any failures + last_run failed/timed_out)');
// deploy-prod has 2 failures (counts), etl-extract last_run failed, health-check has 1 failure (counts), timed-out-job
const failedNames = failedJobs.map(j => j.name).sort();
assertEqual(failedNames, ['deploy-prod', 'etl-extract', 'health-check', 'timed-out-job'].sort(), 'failed filter matches correct jobs');

// --- Failed filter: job that succeeded after failing still shows (has historical failures) ---
const deployProd = failedJobs.find(j => j.name === 'deploy-prod');
assertEqual(deployProd.last_execution.status, 'succeeded', 'deploy-prod last run is succeeded but still shows in failed filter');

// --- Failed filter: job with zero failures excluded ---
const backup = failedJobs.find(j => j.name === 'backup');
assertEqual(backup, undefined, 'backup with 0 failures excluded from failed filter');

// --- Blocked/Waiting filter ---
const blocked = applyFilter(jobs, 'blocked', '', '');
assertEqual(blocked.length, 1, 'blocked filter count');
assertEqual(blocked[0].name, 'etl-transform', 'blocked matches etl-transform');

// --- Scheduled filter ---
const scheduled = applyFilter(jobs, 'scheduled', '', '');
assertEqual(scheduled.length, 5, 'scheduled filter count');

// --- Paused filter ---
const paused = applyFilter(jobs, 'paused', '', '');
assertEqual(paused.length, 1, 'paused filter count');
assertEqual(paused[0].name, 'backup', 'paused matches backup');

// --- Unscheduled filter ---
const unscheduled = applyFilter(jobs, 'unscheduled', '', '');
assertEqual(unscheduled.length, 1, 'unscheduled filter count');
assertEqual(unscheduled[0].name, 'old-job', 'unscheduled matches old-job');

// --- Search filter ---
const searched = applyFilter(jobs, '', 'etl', '');
assertEqual(searched.length, 2, 'search "etl" count');

// --- Search by description ---
const searchDesc = applyFilter(jobs, '', 'production', '');
assertEqual(searchDesc.length, 1, 'search by description count');
assertEqual(searchDesc[0].name, 'deploy-prod', 'search by description matches deploy-prod');

// --- Group filter ---
const etlGroup = applyFilter(jobs, '', '', 'ETL');
assertEqual(etlGroup.length, 3, 'ETL group filter count');

// --- Default group filter ---
const defaultGroup = applyFilter(jobs, '', '', 'Default');
assertEqual(defaultGroup.length, 1, 'Default group matches null group');
assertEqual(defaultGroup[0].name, 'old-job', 'Default group matches old-job');

// --- Combined filters ---
const etlFailed = applyFilter(jobs, 'failed', '', 'ETL');
assertEqual(etlFailed.length, 2, 'failed + ETL group');

const searchRunning = applyFilter(jobs, 'running', 'health', '');
assertEqual(searchRunning.length, 1, 'running + search "health"');

// --- Edge cases ---
const emptyJobs = applyFilter([], 'failed', '', '');
assertEqual(emptyJobs.length, 0, 'empty jobs array returns empty');

const noLastExec = applyFilter([
    { name: 'new-job', status: 'scheduled', depends_on: [], deps_satisfied: true,
      last_execution: null, execution_counts: { total: 0, succeeded: 0, failed: 0 } }
], 'running', '', '');
assertEqual(noLastExec.length, 0, 'running filter handles null last_execution');

const noLastExecFailed = applyFilter([
    { name: 'new-job', status: 'scheduled', depends_on: [], deps_satisfied: true,
      last_execution: null, execution_counts: { total: 0, succeeded: 0, failed: 0 } }
], 'failed', '', '');
assertEqual(noLastExecFailed.length, 0, 'failed filter handles null last_execution with 0 failures');

// --- Results ---
console.log('\n' + (passed + failed) + ' tests, ' + passed + ' passed, ' + failed + ' failed');
process.exit(failed > 0 ? 1 : 0);
