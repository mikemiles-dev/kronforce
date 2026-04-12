// Tests for empty state logic
// Run with: node web/tests/test_empty_states.js

let passed = 0;
let failed = 0;

function assertEqual(actual, expected, msg) {
    if (actual === expected) { passed++; }
    else { failed++; console.error('FAIL:', msg, '- expected:', expected, 'got:', actual); }
}

// Simulate the filter detection logic from jobs.js
function shouldShowFilteredEmpty(statusFilter, searchTerm, groupFilter, timeRange) {
    return !!(statusFilter || searchTerm || groupFilter || timeRange);
}

// No filters — show "create a job" state
assertEqual(shouldShowFilteredEmpty('', '', '', null), false, 'no filters = create state');

// With status filter — show "no matching" state
assertEqual(shouldShowFilteredEmpty('running', '', '', null), true, 'status filter = filtered empty');
assertEqual(shouldShowFilteredEmpty('failed', '', '', null), true, 'failed filter = filtered empty');

// With search — show "no matching" state
assertEqual(shouldShowFilteredEmpty('', 'deploy', '', null), true, 'search = filtered empty');

// With group — show "no matching" state
assertEqual(shouldShowFilteredEmpty('', '', 'ETL', null), true, 'group filter = filtered empty');

// Combined filters
assertEqual(shouldShowFilteredEmpty('scheduled', 'test', 'Monitoring', null), true, 'combined = filtered empty');

// --- Results ---
console.log('\n' + (passed + failed) + ' tests, ' + passed + ' passed, ' + failed + ' failed');
process.exit(failed > 0 ? 1 : 0);
