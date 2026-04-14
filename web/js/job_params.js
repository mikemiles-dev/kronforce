// Kronforce - Job parameters UI functions

var jobParamsDef = [];

function addJobParam() {
    jobParamsDef.push({ name: '', param_type: 'text', required: false, default: '', description: '' });
    renderJobParams();
}

function removeJobParam(idx) {
    jobParamsDef.splice(idx, 1);
    renderJobParams();
}

function renderJobParams() {
    const list = document.getElementById('job-params-list');
    if (!list) return;
    if (jobParamsDef.length === 0) { list.innerHTML = ''; return; }
    let html = '';
    for (let i = 0; i < jobParamsDef.length; i++) {
        const p = jobParamsDef[i];
        html += '<div style="display:flex;gap:6px;align-items:center;margin-bottom:6px;padding:8px;background:var(--bg-tertiary);border-radius:var(--radius);border:1px solid var(--border)">';
        html += '<input type="text" value="' + esc(p.name) + '" placeholder="name" style="width:80px;font-size:11px" onchange="jobParamsDef[' + i + '].name=this.value">';
        html += '<select style="width:80px;font-size:11px" onchange="jobParamsDef[' + i + '].param_type=this.value">';
        for (const t of ['text','number','select','boolean']) {
            html += '<option value="' + t + '"' + (p.param_type === t ? ' selected' : '') + '>' + t + '</option>';
        }
        html += '</select>';
        html += '<input type="text" value="' + esc(p.default || '') + '" placeholder="default" style="width:80px;font-size:11px" onchange="jobParamsDef[' + i + '].default=this.value">';
        html += '<label style="font-size:11px;display:flex;align-items:center;gap:3px;white-space:nowrap"><input type="checkbox"' + (p.required ? ' checked' : '') + ' onchange="jobParamsDef[' + i + '].required=this.checked"> Req</label>';
        html += '<input type="text" value="' + esc(p.description || '') + '" placeholder="description" style="flex:1;font-size:11px" onchange="jobParamsDef[' + i + '].description=this.value">';
        html += '<button class="btn btn-ghost btn-sm" style="color:var(--danger);padding:2px 6px;font-size:14px" onclick="removeJobParam(' + i + ')">&times;</button>';
        html += '</div>';
    }
    list.innerHTML = html;
}

function collectJobParams() {
    const valid = jobParamsDef.filter(p => p.name.trim());
    return valid.length > 0 ? valid : null;
}

function populateJobParams(params) {
    jobParamsDef = params ? JSON.parse(JSON.stringify(params)) : [];
    renderJobParams();
}

// --- Trigger with Params ---
var triggerParamsJobId = null;

function showTriggerParamsModal(jobId, params) {
    triggerParamsJobId = jobId;
    const content = document.getElementById('trigger-params-content');
    let html = '';
    for (const p of params) {
        html += '<div class="form-group" style="margin-bottom:10px">';
        html += '<label>' + esc(p.name) + (p.required ? ' <span style="color:var(--danger)">*</span>' : '') + '</label>';
        if (p.description) html += '<div class="form-hint">' + esc(p.description) + '</div>';
        if (p.param_type === 'boolean') {
            html += '<label style="font-size:12px;display:flex;align-items:center;gap:4px"><input type="checkbox" id="tp-' + esc(p.name) + '"' + (p.default === 'true' ? ' checked' : '') + '> Enabled</label>';
        } else if (p.param_type === 'select' && p.options && p.options.length) {
            html += '<select id="tp-' + esc(p.name) + '">';
            for (const opt of p.options) {
                html += '<option value="' + esc(opt) + '"' + (p.default === opt ? ' selected' : '') + '>' + esc(opt) + '</option>';
            }
            html += '</select>';
        } else if (p.param_type === 'number') {
            html += '<input type="number" id="tp-' + esc(p.name) + '" value="' + esc(p.default || '') + '">';
        } else {
            html += '<input type="text" id="tp-' + esc(p.name) + '" value="' + esc(p.default || '') + '" placeholder="' + esc(p.name) + '">';
        }
        html += '</div>';
    }
    content.innerHTML = html;
    openModal('trigger-params-modal');
}

async function submitTriggerWithParams() {
    if (!triggerParamsJobId) return;
    const job = allJobs.find(j => j.id === triggerParamsJobId);
    const params = {};
    if (job && job.parameters) {
        for (const p of job.parameters) {
            const el = document.getElementById('tp-' + p.name);
            if (!el) continue;
            if (p.param_type === 'boolean') {
                params[p.name] = el.checked ? 'true' : 'false';
            } else {
                params[p.name] = el.value;
            }
        }
    }
    closeModal('trigger-params-modal');
    const btn = document.getElementById('trigger-' + triggerParamsJobId);
    if (btn) btn.classList.add('trigger-pending');
    try {
        await api('POST', '/api/jobs/' + triggerParamsJobId + '/trigger', { params });
        toast('Job triggered with parameters', 'info');
        pollForResult(triggerParamsJobId);
    } catch (e) {
        toast(e.message, 'error');
        if (btn) btn.classList.remove('trigger-pending');
    }
}
