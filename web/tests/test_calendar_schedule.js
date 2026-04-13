// Tests for calendar schedule logic
// Run with: node web/tests/test_calendar_schedule.js

let passed = 0;
let failed = 0;

function assertEqual(actual, expected, msg) {
    if (JSON.stringify(actual) === JSON.stringify(expected)) { passed++; }
    else { failed++; console.error('FAIL:', msg, '- expected:', JSON.stringify(expected), 'got:', JSON.stringify(actual)); }
}

// Simulate fmtSchedule for calendar type
function fmtCalendar(cal) {
    let desc = cal.anchor.replace(/_/g, ' ');
    if (cal.anchor === 'nth_weekday' && cal.weekday) {
        const ordinal = {1:'1st',2:'2nd',3:'3rd',4:'4th'}[cal.nth] || cal.nth + 'th';
        desc = ordinal + ' ' + cal.weekday;
    }
    if (cal.offset_days > 0) desc += ' +' + cal.offset_days + 'd';
    else if (cal.offset_days < 0) desc += ' ' + cal.offset_days + 'd';
    desc += ' @ ' + String(cal.hour || 0).padStart(2, '0') + ':' + String(cal.minute || 0).padStart(2, '0');
    return desc;
}

// Last day of month -2
assertEqual(
    fmtCalendar({ anchor: 'last_day', offset_days: -2, hour: 9, minute: 0 }),
    'last day -2d @ 09:00',
    'last day -2 days at 9am'
);

// First Monday no offset
assertEqual(
    fmtCalendar({ anchor: 'first_monday', offset_days: 0, hour: 8, minute: 30 }),
    'first monday @ 08:30',
    'first monday at 8:30'
);

// 2nd Tuesday
assertEqual(
    fmtCalendar({ anchor: 'nth_weekday', nth: 2, weekday: 'tuesday', offset_days: 0, hour: 10, minute: 0 }),
    '2nd tuesday @ 10:00',
    '2nd tuesday at 10am'
);

// Day 15 with positive offset
assertEqual(
    fmtCalendar({ anchor: 'day_15', offset_days: 1, hour: 17, minute: 0 }),
    'day 15 +1d @ 17:00',
    'day 15 +1 day at 5pm'
);

// Last Friday
assertEqual(
    fmtCalendar({ anchor: 'last_friday', offset_days: 0, hour: 0, minute: 0 }),
    'last friday @ 00:00',
    'last friday midnight'
);

// 4th Thursday (Thanksgiving-style)
assertEqual(
    fmtCalendar({ anchor: 'nth_weekday', nth: 4, weekday: 'thursday', offset_days: 0, hour: 12, minute: 0 }),
    '4th thursday @ 12:00',
    '4th thursday at noon'
);

// Interval schedule format
function fmtInterval(secs) {
    if (secs < 60) return 'every ' + secs + 's';
    if (secs < 3600) return 'every ' + Math.floor(secs / 60) + 'm';
    return 'every ' + Math.floor(secs / 3600) + 'h';
}

assertEqual(fmtInterval(30), 'every 30s', 'interval 30s');
assertEqual(fmtInterval(1800), 'every 30m', 'interval 30min');
assertEqual(fmtInterval(3600), 'every 1h', 'interval 1h');
assertEqual(fmtInterval(7200), 'every 2h', 'interval 2h');

console.log('\n' + (passed + failed) + ' tests, ' + passed + ' passed, ' + failed + ' failed');
process.exit(failed > 0 ? 1 : 0);
