// Kronforce - Agent management
// --- Agents ---
let allAgents = [];
const agentSearch = createSearchFilter({ inputId: 'agent-search-input', clearBtnId: 'agent-search-clear', filterContainerId: 'agent-status-filters', debounceMs: 200, onUpdate: renderAgents });

async function fetchAgents() {
    try {
        allAgents = await api('GET', '/api/agents');
        cacheAgentNames();
        renderAgents();
    } catch (e) {
        console.error('fetchAgents:', e);
    }
}

function renderAgents() {
    const wrap = document.getElementById('agents-table-wrap');

    let html = '';

    if (allAgents.length === 0) {
        wrap.innerHTML = html + renderRichEmptyState({
            icon: '&#128421;',
            title: 'No agents registered',
            description: 'Agents run jobs on remote machines. Start a standard agent with the command above, or build a custom agent in any language.',
            actions: [
                { label: 'View Docs', onclick: "showPage('docs')", primary: true },
            ],
            hint: 'Custom agents poll for work \u2014 build in Python, Go, Node, or any language with an HTTP client.',
        });
        updatePairCommand();
        return;
    }

    // Filter
    let filtered = allAgents;
    if (agentSearch.statusFilter) {
        filtered = filtered.filter(a => a.status === agentSearch.statusFilter);
    }
    if (agentSearch.searchTerm) {
        filtered = filtered.filter(a =>
            a.name.toLowerCase().includes(agentSearch.searchTerm) ||
            a.hostname.toLowerCase().includes(agentSearch.searchTerm) ||
            a.address.toLowerCase().includes(agentSearch.searchTerm) ||
            a.tags.some(t => t.toLowerCase().includes(agentSearch.searchTerm))
        );
    }

    // Summary counts (from all agents, not filtered)
    const online = allAgents.filter(a => a.status === 'online').length;
    const offline = allAgents.filter(a => a.status === 'offline').length;
    const draining = allAgents.filter(a => a.status === 'draining').length;

    html += '<div class="agents-summary">';
    html += '<div class="agents-summary-item"><span class="agent-status-dot online"></span>' + online + ' online</div>';
    if (offline > 0) html += '<div class="agents-summary-item"><span class="agent-status-dot offline"></span>' + offline + ' offline</div>';
    if (draining > 0) html += '<div class="agents-summary-item"><span class="agent-status-dot draining"></span>' + draining + ' draining</div>';
    html += '<div class="agents-summary-item" style="color:var(--text-muted)">' + allAgents.length + ' total</div>';
    if (filtered.length !== allAgents.length) {
        html += '<div class="agents-summary-item" style="color:var(--accent)">' + filtered.length + ' shown</div>';
    }
    html += '</div>';

    if (filtered.length === 0) {
        html += '<div class="agents-empty"><p>No agents match your filters</p></div>';
        wrap.innerHTML = html;
        updatePairCommand();
        return;
    }

    html += '<div class="agents-grid">';
    for (const a of filtered) {
        const isCustom = a.agent_type === 'custom';
        html += '<div class="agent-card ' + a.status + (isCustom ? ' custom-clickable' : '') + '"' + (isCustom ? ' onclick="toggleAgentConfig(\'' + a.id + '\', event)"' : '') + '>';
        html += '<div class="agent-card-header">';
        html += '<span class="agent-name"><span class="agent-icon">' + (isCustom ? '&#9881;' : '&#128421;') + '</span><span class="agent-status-dot ' + a.status + '"></span>' + esc(a.name) + '</span>';
        const typeBadge = isCustom ? '<span class="badge badge-paused">custom</span> ' : '<span class="badge badge-scheduled">standard</span> ';
        html += '<div style="display:flex;align-items:center;gap:6px">' + typeBadge + badge(a.status);
        html += '<button class="btn btn-ghost btn-sm unpair-btn" onclick="event.stopPropagation();unpairAgent(\'' + a.id + '\',\'' + esc(a.name) + '\')" title="Remove agent">Unpair</button>';
        html += '</div></div>';
        html += '<div class="agent-meta">';
        html += infoField('Hostname', esc(a.hostname), 'agent-meta-item');
        html += infoField('Address', esc(a.address) + ':' + a.port, 'agent-meta-item');
        html += infoField('Last Heartbeat', a.last_heartbeat ? fmtDate(a.last_heartbeat) : 'never', 'agent-meta-item');
        html += infoField('Registered', fmtDate(a.registered_at), 'agent-meta-item');
        html += '</div>';
        if (a.tags.length > 0) {
            html += '<div class="agent-tags">';
            for (const t of a.tags) {
                html += '<span class="agent-tag">' + esc(t) + '</span>';
            }
            html += '</div>';
        }
        if (isCustom) {
            const ttCount = (a.task_types && a.task_types.length) || 0;
            html += '<div style="margin-top:8px;padding-top:8px;border-top:1px solid var(--border);font-size:11px;color:var(--text-secondary)">';
            html += '&#9881; ' + ttCount + ' task type' + (ttCount !== 1 ? 's' : '') + ' configured <span style="color:var(--accent);font-size:10px">(click to edit)</span>';
            html += '</div>';
            html += '<div id="agent-config-' + a.id + '" class="agent-config-panel" style="display:none" onclick="event.stopPropagation()"></div>';
        }
        html += '</div>';
    }
    html += '</div>';

    wrap.innerHTML = html;

    // Restore open config panel after re-render
    if (openAgentConfigId) {
        const panel = document.getElementById('agent-config-' + openAgentConfigId);
        if (panel) {
            panel.style.display = '';
            if (!editingAgentTaskTypes[openAgentConfigId]) {
                const agent = allAgents.find(a => a.id === openAgentConfigId);
                editingAgentTaskTypes[openAgentConfigId] = JSON.parse(JSON.stringify(agent?.task_types || []));
            }
            renderTaskTypeEditor(openAgentConfigId);
        }
    }

    // Update pair command with agent key (after DOM is set)
    updatePairCommand();
}

async function unpairAgent(id, name) {
    if (!confirm('Unpair agent "' + name + '"? It will need to re-register to connect again.')) return;
    try {
        await api('DELETE', '/api/agents/' + id);
        toast('Agent "' + name + '" removed');
        fetchAgents();
    } catch (e) {
        toast(e.message, 'error');
    }
}

let editingAgentTaskTypes = {};
let openAgentConfigId = null;

function toggleAgentConfig(agentId, event) {
    if (event.target.closest('.unpair-btn')) return;
    const panel = document.getElementById('agent-config-' + agentId);
    if (!panel) return;
    if (panel.style.display !== 'none') {
        panel.style.display = 'none';
        openAgentConfigId = null;
        return;
    }
    // Close other panels
    document.querySelectorAll('.agent-config-panel').forEach(p => p.style.display = 'none');
    panel.style.display = '';
    openAgentConfigId = agentId;
    // Only reset editing state if not already editing this agent
    if (!editingAgentTaskTypes[agentId]) {
        const agent = allAgents.find(a => a.id === agentId);
        editingAgentTaskTypes[agentId] = JSON.parse(JSON.stringify(agent.task_types || []));
    }
    renderTaskTypeEditor(agentId);
}

function renderTaskTypeEditor(agentId) {
    const panel = document.getElementById('agent-config-' + agentId);
    const taskTypes = editingAgentTaskTypes[agentId] || [];
    let html = '<label class="tt-label">Task Type Definitions</label>';
    taskTypes.forEach((tt, ti) => {
        html += '<div class="tt-section">';
        html += '<div class="tt-header">';
        html += '<input type="text" value="' + esc(tt.name || '') + '" placeholder="Task type name" onchange="ttUpdateName(\'' + agentId + '\',' + ti + ',this.value)" style="font-weight:600">';
        html += '<input type="text" value="' + esc(tt.description || '') + '" placeholder="Description (optional)" onchange="ttUpdateDesc(\'' + agentId + '\',' + ti + ',this.value)" style="flex:2">';
        html += '<button class="btn btn-ghost btn-sm" style="color:var(--danger)" onclick="ttRemoveType(\'' + agentId + '\',' + ti + ')">Remove</button>';
        html += '</div>';
        html += '<label class="tt-label" style="margin-top:4px">Fields</label>';
        (tt.fields || []).forEach((f, fi) => {
            html += '<div class="tt-field-row">';
            html += '<input type="text" value="' + esc(f.name || '') + '" placeholder="name" style="width:80px" onchange="ttUpdateField(\'' + agentId + '\',' + ti + ',' + fi + ',\'name\',this.value)">';
            html += '<input type="text" value="' + esc(f.label || '') + '" placeholder="Label" style="width:100px" onchange="ttUpdateField(\'' + agentId + '\',' + ti + ',' + fi + ',\'label\',this.value)">';
            html += '<select onchange="ttUpdateField(\'' + agentId + '\',' + ti + ',' + fi + ',\'field_type\',this.value);renderTaskTypeEditor(\'' + agentId + '\')" style="width:90px">';
            for (const ft of ['text','textarea','number','select','password']) {
                html += '<option value="' + ft + '"' + (f.field_type === ft ? ' selected' : '') + '>' + ft + '</option>';
            }
            html += '</select>';
            html += '<label style="font-size:11px;display:flex;align-items:center;gap:3px"><input type="checkbox"' + (f.required ? ' checked' : '') + ' onchange="ttUpdateField(\'' + agentId + '\',' + ti + ',' + fi + ',\'required\',this.checked)"> Req</label>';
            html += '<input type="text" value="' + esc(f.placeholder || '') + '" placeholder="placeholder" style="width:100px" onchange="ttUpdateField(\'' + agentId + '\',' + ti + ',' + fi + ',\'placeholder\',this.value)">';
            html += '<button class="btn btn-ghost btn-sm" style="color:var(--danger);padding:2px 6px" onclick="ttRemoveField(\'' + agentId + '\',' + ti + ',' + fi + ')">&times;</button>';
            html += '</div>';
            if (f.field_type === 'select') {
                const optStr = (f.options || []).map(o => o.value + ':' + o.label).join('\n');
                html += '<div style="margin-left:80px;margin-bottom:6px"><textarea placeholder="value:label (one per line)" style="font-size:11px;width:250px;height:50px" onchange="ttUpdateFieldOptions(\'' + agentId + '\',' + ti + ',' + fi + ',this.value)">' + esc(optStr) + '</textarea></div>';
            }
        });
        html += '<button class="btn btn-ghost btn-sm" onclick="ttAddField(\'' + agentId + '\',' + ti + ')" style="font-size:11px">+ Add Field</button>';
        html += '</div>';
    });
    html += '<div style="display:flex;gap:8px;margin-top:8px">';
    html += '<button class="btn btn-ghost btn-sm" onclick="ttAddType(\'' + agentId + '\')">+ Add Task Type</button>';
    html += '<div style="flex:1"></div>';
    html += '<button class="btn btn-primary btn-sm" onclick="ttSave(\'' + agentId + '\')">Save</button>';
    html += '</div>';
    panel.innerHTML = html;
}

function ttUpdateName(agentId, ti, val) { editingAgentTaskTypes[agentId][ti].name = val; }
function ttUpdateDesc(agentId, ti, val) { editingAgentTaskTypes[agentId][ti].description = val; }
function ttUpdateField(agentId, ti, fi, prop, val) { editingAgentTaskTypes[agentId][ti].fields[fi][prop] = val; }
function ttUpdateFieldOptions(agentId, ti, fi, val) {
    editingAgentTaskTypes[agentId][ti].fields[fi].options = val.trim().split('\n').filter(l => l.includes(':')).map(l => {
        const [v, ...rest] = l.split(':');
        return { value: v.trim(), label: rest.join(':').trim() };
    });
}
function ttAddType(agentId) {
    editingAgentTaskTypes[agentId].push({ name: '', description: '', fields: [] });
    renderTaskTypeEditor(agentId);
}
function ttRemoveType(agentId, ti) {
    editingAgentTaskTypes[agentId].splice(ti, 1);
    renderTaskTypeEditor(agentId);
}
function ttAddField(agentId, ti) {
    editingAgentTaskTypes[agentId][ti].fields.push({ name: '', label: '', field_type: 'text', required: false, placeholder: '' });
    renderTaskTypeEditor(agentId);
}
function ttRemoveField(agentId, ti, fi) {
    editingAgentTaskTypes[agentId][ti].fields.splice(fi, 1);
    renderTaskTypeEditor(agentId);
}
async function ttSave(agentId) {
    const taskTypes = editingAgentTaskTypes[agentId] || [];
    for (let i = 0; i < taskTypes.length; i++) {
        if (!taskTypes[i].name.trim()) { toast('Task type ' + (i + 1) + ' needs a name', 'error'); return; }
    }
    try {
        await api('PUT', '/api/agents/' + agentId + '/task-types', { task_types: taskTypes });
        toast('Task types saved');
        // Clear editing state so re-render picks up saved data
        delete editingAgentTaskTypes[agentId];
        fetchAgents();
    } catch (e) {
        toast(e.message, 'error');
    }
}

