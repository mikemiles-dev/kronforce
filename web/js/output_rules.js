// Kronforce - Output rules: extractions, triggers, assertions

function addExtractionRow(name, pattern, type, writeToVar, target) {
    const container = document.getElementById('extractions-container');
    const t = target || 'variable';
    const row = document.createElement('div');
    row.className = 'tt-field-row';
    row.innerHTML =
        '<input type="text" value="' + esc(name || '') + '" placeholder="name" style="width:80px" class="ex-name">' +
        '<input type="text" value="' + esc(pattern || '') + '" placeholder="pattern (regex or $.path)" style="flex:1;min-width:120px" class="ex-pattern">' +
        '<select class="ex-type" style="width:90px"><option value="regex"' + (type === 'jsonpath' ? '' : ' selected') + '>regex</option><option value="jsonpath"' + (type === 'jsonpath' ? ' selected' : '') + '>jsonpath</option></select>' +
        '<select class="ex-target" style="width:90px" title="Where to store extracted value"><option value="variable"' + (t === 'output' ? '' : ' selected') + '>Variable</option><option value="output"' + (t === 'output' ? ' selected' : '') + '>Output</option></select>' +
        '<input type="text" value="' + esc(writeToVar || '') + '" placeholder="write to var" title="Write to global variable (variable target only)" style="width:100px" class="ex-write-var">' +
        '<button class="btn btn-ghost btn-sm" style="color:var(--danger);padding:2px 6px" onclick="this.parentElement.remove()">&times;</button>';
    container.appendChild(row);
    // Show/hide write-var field based on target
    const targetSel = row.querySelector('.ex-target');
    const writeVar = row.querySelector('.ex-write-var');
    function toggleWriteVar() { writeVar.style.display = targetSel.value === 'variable' ? '' : 'none'; }
    targetSel.addEventListener('change', toggleWriteVar);
    toggleWriteVar();
}

function addTriggerRow(pattern, severity) {
    const container = document.getElementById('triggers-container');
    const row = document.createElement('div');
    row.className = 'tt-field-row';
    const sev = severity || 'error';
    row.innerHTML =
        '<input type="text" value="' + esc(pattern || '') + '" placeholder="pattern (regex or substring)" style="flex:1;min-width:150px" class="trig-pattern">' +
        '<select class="trig-severity" style="width:90px">' +
        '<option value="error"' + (sev === 'error' ? ' selected' : '') + '>error</option>' +
        '<option value="warning"' + (sev === 'warning' ? ' selected' : '') + '>warning</option>' +
        '<option value="info"' + (sev === 'info' ? ' selected' : '') + '>info</option>' +
        '<option value="success"' + (sev === 'success' ? ' selected' : '') + '>success</option></select>' +
        '<button class="btn btn-ghost btn-sm" style="color:var(--danger);padding:2px 6px" onclick="this.parentElement.remove()">&times;</button>';
    container.appendChild(row);
}

function addAssertionRow(pattern, message) {
    const container = document.getElementById('assertions-container');
    const row = document.createElement('div');
    row.className = 'tt-field-row';
    row.innerHTML =
        '<input type="text" value="' + esc(pattern || '') + '" placeholder="pattern that MUST appear in output" style="flex:1;min-width:150px" class="assert-pattern">' +
        '<input type="text" value="' + esc(message || '') + '" placeholder="failure message (optional)" style="flex:1;min-width:120px" class="assert-message">' +
        '<button class="btn btn-ghost btn-sm" style="color:var(--danger);padding:2px 6px" onclick="this.parentElement.remove()">&times;</button>';
    container.appendChild(row);
}

function collectOutputRules() {
    const extractions = [];
    document.querySelectorAll('#extractions-container .tt-field-row').forEach(row => {
        const name = row.querySelector('.ex-name').value.trim();
        const pattern = row.querySelector('.ex-pattern').value.trim();
        const type = row.querySelector('.ex-type').value;
        const target = row.querySelector('.ex-target').value || 'variable';
        const write_to_variable = row.querySelector('.ex-write-var').value.trim() || null;
        if (name && pattern) {
            const rule = { name, pattern, type, target };
            if (target === 'variable' && write_to_variable) rule.write_to_variable = write_to_variable;
            extractions.push(rule);
        }
    });
    const triggers = [];
    document.querySelectorAll('#triggers-container .tt-field-row').forEach(row => {
        const pattern = row.querySelector('.trig-pattern').value.trim();
        const severity = row.querySelector('.trig-severity').value;
        if (pattern) triggers.push({ pattern, severity });
    });
    const assertions = [];
    document.querySelectorAll('#assertions-container .tt-field-row').forEach(row => {
        const pattern = row.querySelector('.assert-pattern').value.trim();
        const message = row.querySelector('.assert-message').value.trim();
        if (pattern) assertions.push({ pattern, message: message || null });
    });
    const forward_url = document.getElementById('f-forward-url').value.trim() || null;
    if (extractions.length === 0 && triggers.length === 0 && assertions.length === 0 && !forward_url) return null;
    const rules = { extractions, triggers, assertions };
    if (forward_url) rules.forward_url = forward_url;
    return rules;
}

function populateOutputRules(rules) {
    document.getElementById('extractions-container').innerHTML = '';
    document.getElementById('triggers-container').innerHTML = '';
    document.getElementById('assertions-container').innerHTML = '';
    document.getElementById('f-forward-url').value = '';
    if (!rules) return;
    (rules.extractions || []).forEach(r => addExtractionRow(r.name, r.pattern, r.type, r.write_to_variable, r.target));
    (rules.triggers || []).forEach(t => addTriggerRow(t.pattern, t.severity));
    (rules.assertions || []).forEach(a => addAssertionRow(a.pattern, a.message));
    if (rules.forward_url) document.getElementById('f-forward-url').value = rules.forward_url;
}
