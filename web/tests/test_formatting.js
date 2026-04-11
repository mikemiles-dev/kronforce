// Tests for formatting/utility functions
// Run with: node web/tests/test_formatting.js

let passed = 0;
let failed = 0;

function assert(condition, msg) {
    if (condition) { passed++; }
    else { failed++; console.error('FAIL:', msg); }
}

function assertEqual(actual, expected, msg) {
    if (actual === expected) { passed++; }
    else { failed++; console.error('FAIL:', msg, '- expected:', JSON.stringify(expected), 'got:', JSON.stringify(actual)); }
}

// --- esc (HTML escape) ---
function esc(s) {
    if (s === null || s === undefined) return '';
    return String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

assertEqual(esc('<script>'), '&lt;script&gt;', 'escapes HTML tags');
assertEqual(esc('a&b'), 'a&amp;b', 'escapes ampersand');
assertEqual(esc('"hello"'), '&quot;hello&quot;', 'escapes quotes');
assertEqual(esc(null), '', 'null returns empty');
assertEqual(esc(undefined), '', 'undefined returns empty');
assertEqual(esc(42), '42', 'number coerced to string');

// --- fmtDuration ---
function fmtDuration(start, end) {
    if (!start || !end) return '-';
    const ms = new Date(end) - new Date(start);
    if (ms < 1000) return ms + 'ms';
    if (ms < 60000) return (ms / 1000).toFixed(1) + 's';
    return Math.floor(ms / 60000) + 'm ' + Math.floor((ms % 60000) / 1000) + 's';
}

assertEqual(fmtDuration(null, null), '-', 'null dates return -');
assertEqual(fmtDuration('2026-01-01T00:00:00Z', '2026-01-01T00:00:00.500Z'), '500ms', 'sub-second');
assertEqual(fmtDuration('2026-01-01T00:00:00Z', '2026-01-01T00:00:05Z'), '5.0s', 'seconds');
assertEqual(fmtDuration('2026-01-01T00:00:00Z', '2026-01-01T00:02:30Z'), '2m 30s', 'minutes');

// --- fmtSeconds ---
function fmtSeconds(s) {
    if (s < 60) return s + 's';
    if (s < 3600) return Math.floor(s / 60) + 'm';
    if (s < 86400) return Math.floor(s / 3600) + 'h';
    return Math.floor(s / 86400) + 'd';
}

assertEqual(fmtSeconds(30), '30s', '30 seconds');
assertEqual(fmtSeconds(120), '2m', '2 minutes');
assertEqual(fmtSeconds(7200), '2h', '2 hours');
assertEqual(fmtSeconds(172800), '2d', '2 days');

// --- fmtSchedule ---
function fmtSchedule(sched) {
    if (!sched) return '-';
    if (sched.type === 'cron') return sched.value;
    if (sched.type === 'on_demand') return 'on demand';
    if (sched.type === 'one_shot') return 'one-shot';
    if (sched.type === 'event') return 'event';
    return sched.type;
}

assertEqual(fmtSchedule({ type: 'cron', value: '0 * * * * *' }), '0 * * * * *', 'cron schedule');
assertEqual(fmtSchedule({ type: 'on_demand' }), 'on demand', 'on demand');
assertEqual(fmtSchedule({ type: 'one_shot' }), 'one-shot', 'one shot');
assertEqual(fmtSchedule({ type: 'event' }), 'event', 'event');
assertEqual(fmtSchedule(null), '-', 'null schedule');

// --- pagination function name ---
function paginationFnName(containerId) {
    return '_pag_' + containerId.replace(/-/g, '_');
}

assertEqual(paginationFnName('jobs-pagination'), '_pag_jobs_pagination', 'single hyphen');
assertEqual(paginationFnName('all-execs-pagination'), '_pag_all_execs_pagination', 'multiple hyphens');
assertEqual(paginationFnName('simple'), '_pag_simple', 'no hyphens');

// --- Results ---
console.log('\n' + (passed + failed) + ' tests, ' + passed + ' passed, ' + failed + ' failed');
process.exit(failed > 0 ? 1 : 0);
