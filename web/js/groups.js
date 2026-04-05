// Kronforce - Groups page
// --- Groups Page ---

let groupsPageJobs = [];
let groupsViewMode = localStorage.getItem('kf-groupsView') || 'cards';

function setGroupsView(mode) {
    groupsViewMode = mode;
    localStorage.setItem('kf-groupsView', mode);
    document.querySelectorAll('.groups-view-btn').forEach(b => {
        b.classList.toggle('active', b.id === 'gv-' + mode);
        b.style.background = b.id === 'gv-' + mode ? 'var(--bg-primary)' : '';
    });
    const grid = document.getElementById('groups-grid');
    const mapWrap = document.getElementById('groups-map-wrap');
    const groupFilter = document.getElementById('map-group-filter');
    if (mode === 'map') {
        if (grid) grid.style.display = 'none';
        if (mapWrap) mapWrap.style.display = '';
        if (groupFilter) groupFilter.style.display = '';
        renderMap();
    } else {
        if (grid) grid.style.display = '';
        if (mapWrap) mapWrap.style.display = 'none';
        if (groupFilter) groupFilter.style.display = 'none';
        fetchGroupsPage();
    }
}

async function fetchGroupsPage() {
    try {
        const [groups, jobsRes] = await Promise.all([
            api('GET', '/api/jobs/groups'),
            api('GET', '/api/jobs?per_page=1000'),
        ]);
        const jobs = jobsRes.data;
        groupsPageJobs = jobs;

        // Collect jobs per group
        const jobsByGroup = {};
        for (const j of jobs) {
            const g = j.group || 'Default';
            if (!jobsByGroup[g]) jobsByGroup[g] = [];
            jobsByGroup[g].push(j);
        }

        const grid = document.getElementById('groups-grid');
        if (!grid) return;

        if (jobs.length === 0) {
            grid.innerHTML = renderRichEmptyState({
                icon: '&#128193;',
                title: 'No jobs yet',
                description: 'Create jobs first, then organize them into groups.',
                actions: [
                    { label: 'Create a Job', onclick: 'openCreateModal()', primary: true },
                ],
            });
            return;
        }

        // Build the list of groups to display — merge API groups with groups found in job data
        const allGroups = new Set(groups);
        for (const g of Object.keys(jobsByGroup)) {
            allGroups.add(g);
        }
        const sortedGroups = [...allGroups].sort((a, b) => {
            // Default always first
            if (a === 'Default') return -1;
            if (b === 'Default') return 1;
            return a.localeCompare(b);
        });

        // Restore view toggle state
        document.querySelectorAll('.groups-view-btn').forEach(b => {
            b.classList.toggle('active', b.id === 'gv-' + groupsViewMode);
            b.style.background = b.id === 'gv-' + groupsViewMode ? 'var(--bg-primary)' : '';
        });

        if (groupsViewMode === 'pipeline') {
            renderPipelineView(sortedGroups, jobsByGroup);
        } else {
            renderCardsView(sortedGroups, jobsByGroup);
        }
    } catch (e) {
        console.error('fetchGroupsPage:', e);
    }
}

function renderCardsView(sortedGroups, jobsByGroup) {
    const grid = document.getElementById('groups-grid');
    grid.className = 'groups-grid';
    let html = '';
    for (const g of sortedGroups) {
        const groupJobs = jobsByGroup[g] || [];
        const count = groupJobs.length;
        const color = groupColor(g);
        html += '<div class="group-card">';
        html += '<div class="group-card-header" style="cursor:pointer" onclick="navToGroupJobs(\'' + esc(g) + '\')">';
        html += '<span class="group-card-dot" style="background:' + color + '"></span>';
        html += '<span class="group-card-name">' + esc(g) + '</span>';
        html += '<span style="margin-left:auto;font-size:11px;color:var(--accent)">&rarr;</span>';
        html += '</div>';
        html += '<div class="group-card-count">' + count + ' job' + (count !== 1 ? 's' : '') + '</div>';
        if (groupJobs.length > 0) {
            html += '<div class="group-card-jobs">';
            for (const j of groupJobs.slice(0, 5)) {
                html += '<div class="group-card-job">' + esc(j.name) + '</div>';
            }
            if (groupJobs.length > 5) {
                html += '<div class="group-card-job" style="color:var(--text-muted)">...and ' + (groupJobs.length - 5) + ' more</div>';
            }
            html += '</div>';
        }
        html += '<div class="group-card-actions">';
        html += '<button class="btn btn-ghost btn-sm" onclick="renameGroup(\'' + esc(g) + '\')">Rename</button>';
        html += '<button class="btn btn-ghost btn-sm" style="color:var(--danger)" onclick="deleteGroup(\'' + esc(g) + '\')">Delete</button>';
        html += '</div>';
        html += '</div>';
    }
    grid.innerHTML = html;
}

function renderPipelineView(sortedGroups, jobsByGroup) {
    const grid = document.getElementById('groups-grid');
    grid.className = '';
    let html = '';

    for (const g of sortedGroups) {
        const groupJobs = jobsByGroup[g] || [];
        if (groupJobs.length === 0) continue;
        const color = groupColor(g);

        // Sort jobs: roots first (no deps in this group), then by dependency chain
        const inGroup = new Set(groupJobs.map(j => j.id));
        const depMap = {};
        for (const j of groupJobs) {
            depMap[j.id] = (j.depends_on || []).filter(d => inGroup.has(d.job_id)).map(d => d.job_id);
        }
        // Topological sort
        const sorted = [];
        const visited = new Set();
        function visit(id) {
            if (visited.has(id)) return;
            visited.add(id);
            for (const pid of (depMap[id] || [])) visit(pid);
            sorted.push(id);
        }
        for (const j of groupJobs) visit(j.id);
        const jobMap = {};
        for (const j of groupJobs) jobMap[j.id] = j;

        // Pipeline header
        html += '<div style="margin-bottom:20px">';
        html += '<div style="display:flex;align-items:center;gap:8px;margin-bottom:10px;cursor:pointer" onclick="navToGroupJobs(\'' + esc(g) + '\')">';
        html += '<span style="width:12px;height:12px;border-radius:50%;background:' + color + ';flex-shrink:0"></span>';
        html += '<strong style="font-size:14px">' + esc(g) + '</strong>';
        html += '<span style="font-size:12px;color:var(--text-muted)">' + groupJobs.length + ' stage' + (groupJobs.length !== 1 ? 's' : '') + '</span>';
        html += '</div>';

        // Build set of which jobs have a dependency arrow FROM the previous job in sorted order
        const hasArrowFrom = new Set();
        for (const j of groupJobs) {
            for (const d of (j.depends_on || [])) {
                if (inGroup.has(d.job_id)) hasArrowFrom.add(j.id);
            }
        }

        // Render stage card helper
        function stageCard(j) {
            const last = j.last_execution;
            const status = last ? last.status : 'idle';
            let bg, border, statusIcon;
            if (status === 'succeeded') { bg = 'rgba(46,204,113,0.12)'; border = '#2ecc71'; statusIcon = '\u2714'; }
            else if (status === 'failed' || status === 'timed_out') { bg = 'rgba(224,82,82,0.12)'; border = '#e05252'; statusIcon = '\u2718'; }
            else if (status === 'running') { bg = 'rgba(62,139,255,0.12)'; border = '#3e8bff'; statusIcon = '\u25B6'; }
            else if (status === 'pending_approval') { bg = 'rgba(230,168,23,0.12)'; border = '#e6a817'; statusIcon = '\u23F3'; }
            else { bg = 'var(--bg-tertiary)'; border = 'var(--border)'; statusIcon = '\u25CB'; }
            let s = '<div style="background:' + bg + ';border:2px solid ' + border + ';border-radius:8px;padding:10px 14px;min-width:130px;cursor:pointer;flex-shrink:0;text-align:center" onclick="showJobDetail(\'' + j.id + '\')">';
            s += '<div style="font-size:18px;margin-bottom:4px">' + statusIcon + '</div>';
            s += '<div style="font-size:12px;font-weight:600;margin-bottom:2px;white-space:nowrap">' + esc(j.name) + '</div>';
            s += '<div style="font-size:10px;color:var(--text-muted)">' + status + '</div>';
            if (last && last.finished_at) s += '<div style="font-size:9px;color:var(--text-muted)">' + fmtDate(last.finished_at) + '</div>';
            s += '</div>';
            return s;
        }

        const arrow = '<div style="display:flex;align-items:center;flex-shrink:0"><div style="width:20px;height:3px;background:var(--accent);border-radius:2px"></div><div style="width:0;height:0;border-top:6px solid transparent;border-bottom:6px solid transparent;border-left:8px solid var(--accent)"></div></div>';

        // Pipeline stages — only show arrows between jobs with actual dependencies
        html += '<div style="display:flex;align-items:center;gap:6px;overflow-x:auto;padding:4px 0;flex-wrap:wrap">';
        for (let i = 0; i < sorted.length; i++) {
            const j = jobMap[sorted[i]];
            if (!j) continue;

            // Arrow only if this job depends on another job in this group
            if (i > 0 && hasArrowFrom.has(j.id)) {
                html += arrow;
            } else if (i > 0) {
                // Gap separator for independent jobs (no arrow)
                html += '<div style="width:8px;flex-shrink:0"></div>';
            }

            html += stageCard(j);
        }
        html += '</div></div>';
    }

    grid.innerHTML = html;
}

async function createNewGroup() {
    const name = prompt('Enter new group name:');
    if (!name || !name.trim()) return;
    const trimmed = name.trim();
    if (cachedGroups.includes(trimmed)) {
        toast('Group "' + trimmed + '" already exists', 'error');
        return;
    }
    try {
        await api('POST', '/api/jobs/groups', { name: trimmed });
        toast('Group "' + trimmed + '" created');
        fetchGroupsPage();
        fetchGroups();
    } catch (e) {
        toast('Error: ' + e.message, 'error');
    }
}

function navToGroupJobs(group) {
    groupFilter = group;
    showPage('jobs');
    const sel = document.getElementById('group-filter');
    if (sel) sel.value = group;
    fetchJobs(true);
}

async function renameGroup(oldName) {
    const newName = prompt('Rename group "' + oldName + '" to:', oldName);
    if (!newName || newName === oldName) return;
    try {
        await api('PUT', '/api/jobs/rename-group', { old_name: oldName, new_name: newName });
        toast('Group renamed to "' + newName + '"');
        fetchGroupsPage();
        fetchGroups();
    } catch (e) {
        toast('Error: ' + e.message, 'error');
    }
}

async function deleteGroup(name) {
    if (name === 'Default') {
        toast('The Default group cannot be deleted', 'error');
        return;
    }
    if (!confirm('Delete group "' + name + '"? All jobs in this group will be moved to the Default group.')) return;
    try {
        await api('PUT', '/api/jobs/rename-group', { old_name: name, new_name: 'Default' });
        toast('Group "' + name + '" deleted — jobs moved to Default');
        fetchGroupsPage();
        fetchGroups();
    } catch (e) {
        toast('Error: ' + e.message, 'error');
    }
}
