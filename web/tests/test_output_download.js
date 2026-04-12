// Tests for output download filename generation and trigger params
// Run with: node web/tests/test_output_download.js

let passed = 0;
let failed = 0;

function assertEqual(actual, expected, msg) {
    if (actual === expected) { passed++; }
    else { failed++; console.error('FAIL:', msg, '- expected:', expected, 'got:', actual); }
}

// Download filename generation (from executions.js downloadOutput)
function generateFilename(execId, label) {
    return (execId ? execId.slice(0, 8) : 'output') + '-' + label + '.log';
}

assertEqual(generateFilename('abcd1234-5678-90ab', 'output'), 'abcd1234-output.log', 'exec id prefix + output');
assertEqual(generateFilename('abcd1234-5678-90ab', 'error'), 'abcd1234-error.log', 'exec id prefix + error');
assertEqual(generateFilename(null, 'output'), 'output-output.log', 'null exec id fallback');
assertEqual(generateFilename('', 'output'), 'output-output.log', 'empty exec id fallback');

// Trigger params form value extraction (simulated)
function collectParams(paramDefs, values) {
    const params = {};
    for (const p of paramDefs) {
        const val = values[p.name];
        if (val !== undefined) {
            if (p.param_type === 'boolean') {
                params[p.name] = val ? 'true' : 'false';
            } else {
                params[p.name] = val;
            }
        }
    }
    return params;
}

const defs = [
    { name: 'version', param_type: 'text', required: true, default: '1.0' },
    { name: 'env', param_type: 'select', required: false, default: 'staging' },
    { name: 'force', param_type: 'boolean', required: false, default: 'false' },
    { name: 'count', param_type: 'number', required: false },
];

const vals = { version: '2.0', env: 'production', force: true, count: '5' };
const result = collectParams(defs, vals);

assertEqual(result.version, '2.0', 'text param');
assertEqual(result.env, 'production', 'select param');
assertEqual(result.force, 'true', 'boolean param true');
assertEqual(result.count, '5', 'number param');

// Boolean false
const vals2 = { version: '1.0', force: false };
const result2 = collectParams(defs, vals2);
assertEqual(result2.force, 'false', 'boolean param false');

// Missing optional params not included
const vals3 = { version: '1.0' };
const result3 = collectParams(defs, vals3);
assertEqual(result3.version, '1.0', 'only provided param');
assertEqual(result3.env, undefined, 'missing optional not included');

console.log('\n' + (passed + failed) + ' tests, ' + passed + ' passed, ' + failed + ' failed');
process.exit(failed > 0 ? 1 : 0);
