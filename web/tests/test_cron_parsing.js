// Tests for cron reverse parsing (expression → UI fields)
// Run with: node web/tests/test_cron_parsing.js

let passed = 0;
let failed = 0;

function assertEqual(actual, expected, msg) {
    if (JSON.stringify(actual) === JSON.stringify(expected)) { passed++; }
    else { failed++; console.error('FAIL:', msg, '- expected:', JSON.stringify(expected), 'got:', JSON.stringify(actual)); }
}

// detectCronMode from modals.js
function detectCronMode(val) {
    if (val === '*') return { mode: 'every', val: '*' };
    if (val.startsWith('*/')) return { mode: 'step', val: val.slice(2) };
    if (val.includes('-')) return { mode: 'range', val: val };
    return { mode: 'fixed', val: val };
}

// Simulate parseCronToUI — returns field states
function parseCron(expr) {
    const parts = expr.split(/\s+/);
    if (parts.length !== 6) return null;
    const [sec, min, hr, dom, mon, dow] = parts;
    return {
        sec: detectCronMode(sec),
        min: detectCronMode(min),
        hr: detectCronMode(hr),
        dom: detectCronMode(dom),
        mon: detectCronMode(mon),
        dow: dow === '*' ? [] : dow.split(','),
    };
}

// Every minute
let p = parseCron('0 * * * * *');
assertEqual(p.sec, { mode: 'fixed', val: '0' }, 'every min: sec=fixed 0');
assertEqual(p.min, { mode: 'every', val: '*' }, 'every min: min=every');
assertEqual(p.hr, { mode: 'every', val: '*' }, 'every min: hr=every');
assertEqual(p.dow, [], 'every min: no dow');

// Every 5 minutes
p = parseCron('0 */5 * * * *');
assertEqual(p.min, { mode: 'step', val: '5' }, 'every 5 min: min=step 5');

// Daily at 9:30
p = parseCron('0 30 9 * * *');
assertEqual(p.min, { mode: 'fixed', val: '30' }, 'daily: min=fixed 30');
assertEqual(p.hr, { mode: 'fixed', val: '9' }, 'daily: hr=fixed 9');

// Weekdays at 8am
p = parseCron('0 0 8 * * 1,2,3,4,5');
assertEqual(p.dow, ['1', '2', '3', '4', '5'], 'weekdays: dow parsed');
assertEqual(p.hr, { mode: 'fixed', val: '8' }, 'weekdays: hr=8');

// First of month
p = parseCron('0 0 0 1 * *');
assertEqual(p.dom, { mode: 'fixed', val: '1' }, 'first of month: dom=1');

// 1st and 15th
p = parseCron('0 0 0 1,15 * *');
assertEqual(p.dom, { mode: 'fixed', val: '1,15' }, '1st and 15th: dom=1,15');

// Hour range 9-17
p = parseCron('0 0 9-17 * * *');
assertEqual(p.hr, { mode: 'range', val: '9-17' }, 'hour range: hr=9-17');

// Every 10 seconds
p = parseCron('*/10 * * * * *');
assertEqual(p.sec, { mode: 'step', val: '10' }, 'every 10 sec: sec=step 10');

// Specific months
p = parseCron('0 0 0 * 1,7 *');
assertEqual(p.mon, { mode: 'fixed', val: '1,7' }, 'Jan/Jul: mon=1,7');

// Invalid expression
assertEqual(parseCron('bad'), null, 'invalid returns null');
assertEqual(parseCron('1 2 3'), null, 'too few fields returns null');

// Sunday only
p = parseCron('0 0 0 * * 0');
assertEqual(p.dow, ['0'], 'sunday: dow=[0]');

console.log('\n' + (passed + failed) + ' tests, ' + passed + ' passed, ' + failed + ' failed');
process.exit(failed > 0 ? 1 : 0);
