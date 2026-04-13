// Tests for URL state encoding/decoding (share + hash persistence)
// Run with: node web/tests/test_url_state.js

let passed = 0;
let failed = 0;

function assertEqual(actual, expected, msg) {
    if (JSON.stringify(actual) === JSON.stringify(expected)) { passed++; }
    else { failed++; console.error('FAIL:', msg, '- expected:', JSON.stringify(expected), 'got:', JSON.stringify(actual)); }
}

// parseHashParams from app.js
function parseHashParams(hash) {
    var qIdx = hash.indexOf('?');
    if (qIdx === -1) return {};
    var params = {};
    hash.slice(qIdx + 1).split('&').forEach(function(p) {
        var kv = p.split('=');
        if (kv.length === 2) params[kv[0]] = decodeURIComponent(kv[1]);
    });
    return params;
}

// Build hash params (simulates updateHash logic)
function buildHashParams(filter, group, search) {
    var params = [];
    if (filter) params.push('filter=' + encodeURIComponent(filter));
    if (group) params.push('group=' + encodeURIComponent(group));
    if (search) params.push('search=' + encodeURIComponent(search));
    return params.length > 0 ? '?' + params.join('&') : '';
}

// --- Parse tests ---
assertEqual(parseHashParams('#/jobs'), {}, 'no params');
assertEqual(parseHashParams('#/jobs?filter=failed'), { filter: 'failed' }, 'filter only');
assertEqual(parseHashParams('#/jobs?group=ETL'), { group: 'ETL' }, 'group only');
assertEqual(parseHashParams('#/jobs?filter=running&group=ETL&search=deploy'), { filter: 'running', group: 'ETL', search: 'deploy' }, 'all params');
assertEqual(parseHashParams('#/jobs/stages?group=ETL'), { group: 'ETL' }, 'tabs with group');
assertEqual(parseHashParams('#/executions?filter=failed&time=60'), { filter: 'failed', time: '60' }, 'executions with time');
assertEqual(parseHashParams('#/events?filter=error&search=agent'), { filter: 'error', search: 'agent' }, 'events');
assertEqual(parseHashParams('#/agents?filter=online'), { filter: 'online' }, 'agents');

// --- Build tests ---
assertEqual(buildHashParams('', '', ''), '', 'no params builds empty');
assertEqual(buildHashParams('failed', '', ''), '?filter=failed', 'filter only');
assertEqual(buildHashParams('', 'ETL', ''), '?group=ETL', 'group only');
assertEqual(buildHashParams('', '', 'deploy'), '?search=deploy', 'search only');
assertEqual(buildHashParams('running', 'ETL', 'deploy'), '?filter=running&group=ETL&search=deploy', 'all params');

// --- Round-trip tests (build then parse) ---
var hash1 = '#/jobs' + buildHashParams('failed', 'Monitoring', 'health');
var parsed1 = parseHashParams(hash1);
assertEqual(parsed1.filter, 'failed', 'round-trip filter');
assertEqual(parsed1.group, 'Monitoring', 'round-trip group');
assertEqual(parsed1.search, 'health', 'round-trip search');

// Group with special chars
var hash2 = '#/jobs' + buildHashParams('', 'ETL Pipeline', '');
var parsed2 = parseHashParams(hash2);
assertEqual(parsed2.group, 'ETL Pipeline', 'group with space round-trips');

// Empty filter persists group
var hash3 = '#/jobs' + buildHashParams('', 'Deploys', '');
var parsed3 = parseHashParams(hash3);
assertEqual(parsed3.filter, undefined, 'no filter set');
assertEqual(parsed3.group, 'Deploys', 'group persists without filter');

console.log('\n' + (passed + failed) + ' tests, ' + passed + ' passed, ' + failed + ' failed');
process.exit(failed > 0 ? 1 : 0);
