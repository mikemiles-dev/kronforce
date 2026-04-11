// Tests for cron builder logic
// Run with: node web/tests/test_cron_builder.js

let passed = 0;
let failed = 0;

function assert(condition, msg) {
    if (condition) { passed++; }
    else { failed++; console.error('FAIL:', msg); }
}

function assertEqual(actual, expected, msg) {
    if (actual === expected) { passed++; }
    else { failed++; console.error('FAIL:', msg, '- expected:', expected, 'got:', actual); }
}

// --- cronFieldVal ---
function cronFieldVal(mode, val) {
    if (mode === 'every') return '*';
    if (mode === 'step') return '*/' + (val || '1');
    return val || '*';
}

assertEqual(cronFieldVal('every', ''), '*', 'every mode returns *');
assertEqual(cronFieldVal('step', '5'), '*/5', 'step mode returns */N');
assertEqual(cronFieldVal('step', ''), '*/1', 'step mode with empty defaults to */1');
assertEqual(cronFieldVal('fixed', '30'), '30', 'fixed mode returns value');
assertEqual(cronFieldVal('fixed', ''), '*', 'fixed mode with empty returns *');
assertEqual(cronFieldVal('range', '9-17'), '9-17', 'range mode returns value');

// --- detectCronMode ---
function detectCronMode(val) {
    if (val === '*') return { mode: 'every', val: '*' };
    if (val.startsWith('*/')) return { mode: 'step', val: val.slice(2) };
    if (val.includes('-')) return { mode: 'range', val: val };
    return { mode: 'fixed', val: val };
}

assertEqual(detectCronMode('*').mode, 'every', 'detect * as every');
assertEqual(detectCronMode('*/5').mode, 'step', 'detect */5 as step');
assertEqual(detectCronMode('*/5').val, '5', 'detect */5 val');
assertEqual(detectCronMode('9-17').mode, 'range', 'detect 9-17 as range');
assertEqual(detectCronMode('9-17').val, '9-17', 'detect 9-17 val');
assertEqual(detectCronMode('0').mode, 'fixed', 'detect 0 as fixed');
assertEqual(detectCronMode('1,15').mode, 'fixed', 'detect 1,15 as fixed');

// --- Build full expressions ---
function buildExpr(sec, min, hr, dom, mon, dow) {
    return sec + ' ' + min + ' ' + hr + ' ' + dom + ' ' + mon + ' ' + dow;
}

assertEqual(buildExpr('0', '*', '*', '*', '*', '*'), '0 * * * * *', 'every minute');
assertEqual(buildExpr('0', '*/5', '*', '*', '*', '*'), '0 */5 * * * *', 'every 5 minutes');
assertEqual(buildExpr('0', '0', '9', '*', '*', '1,2,3,4,5'), '0 0 9 * * 1,2,3,4,5', 'weekdays at 9am');
assertEqual(buildExpr('0', '30', '*/2', '*', '*', '*'), '0 30 */2 * * *', 'every 2 hours at :30');
assertEqual(buildExpr('0', '0', '0', '1', '*', '*'), '0 0 0 1 * *', 'first of month midnight');
assertEqual(buildExpr('0', '0', '0', '1,15', '*', '*'), '0 0 0 1,15 * *', '1st and 15th midnight');
assertEqual(buildExpr('*/10', '*', '*', '*', '*', '*'), '*/10 * * * * *', 'every 10 seconds');
assertEqual(buildExpr('0', '0', '9-17', '*', '*', '*'), '0 0 9-17 * * *', 'hourly 9am-5pm');
assertEqual(buildExpr('0', '0', '0', '*', '1,7', '*'), '0 0 0 * 1,7 *', 'Jan and Jul midnight');

// --- Results ---
console.log('\n' + (passed + failed) + ' tests, ' + passed + ' passed, ' + failed + ' failed');
process.exit(failed > 0 ? 1 : 0);
