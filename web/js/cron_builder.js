// Kronforce - Cron builder UI functions

function switchCronMode(mode, btn) {
    btn.parentElement.querySelectorAll('.output-tab').forEach(t => t.classList.remove('active'));
    btn.classList.add('active');
    document.getElementById('cron-builder').style.display = mode === 'builder' ? '' : 'none';
    document.getElementById('cron-raw').style.display = mode === 'raw' ? '' : 'none';
    if (mode === 'raw') {
        // Sync builder -> raw
        document.getElementById('f-cron').value = document.getElementById('cb-preview').textContent;
    } else {
        // Try to parse raw -> builder
        parseCronToUI(document.getElementById('f-cron').value.trim());
    }
}

function toggleDow(btn) {
    btn.classList.toggle('active');
    buildCronFromUI();
}

function cronFieldVal(mode, val) {
    if (mode === 'every') return '*';
    if (mode === 'step') return '*/' + (val || '1');
    return val || '*';
}

function buildCronFromUI() {
    const sec = cronFieldVal(
        document.getElementById('cb-sec-mode').value,
        document.getElementById('cb-sec-val').value
    );
    const min = cronFieldVal(
        document.getElementById('cb-min-mode').value,
        document.getElementById('cb-min-val').value
    );
    const hr = cronFieldVal(
        document.getElementById('cb-hr-mode').value,
        document.getElementById('cb-hr-val').value
    );
    const dom = cronFieldVal(
        document.getElementById('cb-dom-mode').value,
        document.getElementById('cb-dom-val').value
    );
    const mon = cronFieldVal(
        document.getElementById('cb-mon-mode').value,
        document.getElementById('cb-mon-val').value
    );

    const selectedDow = Array.from(document.querySelectorAll('.cron-dow.active')).map(b => b.dataset.dow);
    const dow = selectedDow.length > 0 ? selectedDow.join(',') : '*';

    const expr = sec + ' ' + min + ' ' + hr + ' ' + dom + ' ' + mon + ' ' + dow;

    // Build description
    const dayNames = {0:'Sun',1:'Mon',2:'Tue',3:'Wed',4:'Thu',5:'Fri',6:'Sat'};
    let parts = [];
    if (sec !== '0' && sec !== '*') parts.push('sec=' + sec);
    if (min === '*') parts.push('every minute');
    else if (min.startsWith('*/')) parts.push('every ' + min.slice(2) + ' min');
    else parts.push('at :' + String(min).padStart(2, '0'));
    if (hr !== '*') {
        if (hr.startsWith('*/')) parts.push('every ' + hr.slice(2) + ' hours');
        else if (hr.includes('-')) parts.push('hours ' + hr);
        else parts.push(String(hr).padStart(2, '0') + 'h');
    }
    if (dom !== '*') parts.push('day ' + dom);
    if (mon !== '*') parts.push('month ' + mon);
    if (dow !== '*') parts.push(selectedDow.map(d => dayNames[d]).join(','));

    document.getElementById('cb-preview').textContent = expr;
    document.getElementById('cb-description').textContent = parts.join(', ');
    document.getElementById('f-cron').value = expr;
}

function detectCronMode(val) {
    if (val === '*') return { mode: 'every', val: '*' };
    if (val.startsWith('*/')) return { mode: 'step', val: val.slice(2) };
    if (val.includes('-')) return { mode: 'range', val: val };
    return { mode: 'fixed', val: val };
}

function parseCronToUI(expr) {
    // Reset dow buttons
    document.querySelectorAll('.cron-dow').forEach(b => b.classList.remove('active'));

    if (!expr) {
        // Defaults
        document.getElementById('cb-sec-mode').value = 'fixed';
        document.getElementById('cb-sec-val').value = '0';
        document.getElementById('cb-min-mode').value = 'every';
        document.getElementById('cb-min-val').value = '*';
        document.getElementById('cb-hr-mode').value = 'every';
        document.getElementById('cb-hr-val').value = '*';
        document.getElementById('cb-dom-mode').value = 'every';
        document.getElementById('cb-dom-val').value = '*';
        document.getElementById('cb-mon-mode').value = 'every';
        document.getElementById('cb-mon-val').value = '*';
        buildCronFromUI();
        return;
    }
    const parts = expr.split(/\s+/);
    if (parts.length !== 6) { buildCronFromUI(); return; }

    const [sec, min, hr, dom, mon, dow] = parts;

    const s = detectCronMode(sec);
    document.getElementById('cb-sec-mode').value = s.mode;
    document.getElementById('cb-sec-val').value = s.val;

    const m = detectCronMode(min);
    document.getElementById('cb-min-mode').value = m.mode;
    document.getElementById('cb-min-val').value = m.val;

    const h = detectCronMode(hr);
    document.getElementById('cb-hr-mode').value = h.mode;
    document.getElementById('cb-hr-val').value = h.val;

    const d = detectCronMode(dom);
    document.getElementById('cb-dom-mode').value = d.mode;
    document.getElementById('cb-dom-val').value = d.val;

    const mo = detectCronMode(mon);
    document.getElementById('cb-mon-mode').value = mo.mode;
    document.getElementById('cb-mon-val').value = mo.val;

    if (dow !== '*') {
        dow.split(',').forEach(v => {
            const btn = document.querySelector('.cron-dow[data-dow="' + v.trim() + '"]');
            if (btn) btn.classList.add('active');
        });
    }

    buildCronFromUI();
}

function updateCronPreviewFromRaw() {
    // Just sync the raw input — don't update builder to avoid loops
}

// Legacy compat — old code called these
function updateCronOptions() {}

function getCronValue() {
    return document.getElementById('f-cron').value.trim() || document.getElementById('cb-preview').textContent;
}
