// Kronforce - Groups page
// --- Groups Page ---

let groupsPageJobs = [];
let groupsViewMode = localStorage.getItem('kf-groupsView') || 'cards';
let plGroupPickerOpen = false;

function togglePipelineGroupPicker() {
    const pop = document.getElementById('pl-group-picker-popover');
    if (!pop) return;
    plGroupPickerOpen = !plGroupPickerOpen;
    pop.style.display = plGroupPickerOpen ? '' : 'none';
    if (plGroupPickerOpen) {
        document.getElementById('pl-group-picker-search').value = '';
        populatePipelineGroupList();
        document.getElementById('pl-group-picker-search').focus();
        setTimeout(function() { document.addEventListener('click', closePipelineGroupPickerOutside); }, 10);
    } else {
        document.removeEventListener('click', closePipelineGroupPickerOutside);
    }
}

function closePipelineGroupPickerOutside(e) {
    const wrap = document.getElementById('pl-group-picker-wrap');
    if (wrap && !wrap.contains(e.target)) {
        plGroupPickerOpen = false;
        document.getElementById('pl-group-picker-popover').style.display = 'none';
        document.removeEventListener('click', closePipelineGroupPickerOutside);
    }
}

function populatePipelineGroupList() {
    const list = document.getElementById('pl-group-picker-list');
    if (!list) return;
    const search = (document.getElementById('pl-group-picker-search').value || '').trim();
    const searchLower = search.toLowerCase();
    const groups = (typeof cachedGroups !== 'undefined' ? cachedGroups : []).filter(function(g) {
        return !searchLower || g.toLowerCase().includes(searchLower);
    });
    let html = '<div class="group-picker-item" onclick="selectPipelineGroup(\'\')" style="font-style:italic">All Groups</div>';
    for (const g of groups) {
        html += '<div class="group-picker-item" onclick="selectPipelineGroup(\'' + escAttr(g) + '\')">' + esc(g) + '</div>';
    }
    var exactMatch = search && groups.some(function(g) { return g.toLowerCase() === searchLower; });
    if (search && !exactMatch) {
        html += '<div class="group-picker-item" onclick="createAndSelectPipelineGroup(\'' + escAttr(search) + '\')" style="color:var(--accent)">+ Create "' + esc(search) + '"</div>';
    }
    if (!search && groups.length > 0) {
        html += '<div style="padding:6px 12px;font-size:11px;color:var(--text-muted);border-top:1px solid var(--border)">Type a new name to create a group</div>';
    }
    list.innerHTML = html;
}

async function createAndSelectPipelineGroup(name) {
    try {
        await api('POST', '/api/jobs/groups', { name: name });
        if (typeof fetchGroups === 'function') await fetchGroups();
        selectPipelineGroup(name);
        toast('Group "' + name + '" created');
    } catch (e) { toast(e.message, 'error'); }
}

function filterPipelineGroupPicker() {
    populatePipelineGroupList();
}

// --- Designer Group Picker ---
let dsGroupPickerOpen = false;

function toggleDesignerGroupPicker() {
    const pop = document.getElementById('ds-group-picker-popover');
    if (!pop) return;
    dsGroupPickerOpen = !dsGroupPickerOpen;
    pop.style.display = dsGroupPickerOpen ? '' : 'none';
    if (dsGroupPickerOpen) {
        document.getElementById('ds-group-picker-search').value = '';
        populateDesignerGroupList();
        document.getElementById('ds-group-picker-search').focus();
        setTimeout(function() { document.addEventListener('click', closeDesignerGroupPickerOutside); }, 10);
    } else {
        document.removeEventListener('click', closeDesignerGroupPickerOutside);
    }
}

function closeDesignerGroupPickerOutside(e) {
    const wrap = document.getElementById('ds-group-picker-wrap');
    if (wrap && !wrap.contains(e.target)) {
        dsGroupPickerOpen = false;
        document.getElementById('ds-group-picker-popover').style.display = 'none';
        document.removeEventListener('click', closeDesignerGroupPickerOutside);
    }
}

function populateDesignerGroupList() {
    const list = document.getElementById('ds-group-picker-list');
    if (!list) return;
    const search = (document.getElementById('ds-group-picker-search').value || '').trim();
    const searchLower = search.toLowerCase();
    const groups = (typeof cachedGroups !== 'undefined' ? cachedGroups : []).filter(function(g) {
        return !searchLower || g.toLowerCase().includes(searchLower);
    });
    let html = '';
    for (const g of groups) {
        html += '<div class="group-picker-item" onclick="selectDesignerGroup(\'' + escAttr(g) + '\')">' + esc(g) + '</div>';
    }
    // Show create option if user typed something that doesn't exactly match an existing group
    var exactMatch = search && groups.some(function(g) { return g.toLowerCase() === searchLower; });
    if (search && !exactMatch) {
        html += '<div class="group-picker-item" onclick="createAndSelectDesignerGroup(\'' + escAttr(search) + '\')" style="color:var(--accent)">+ Create "' + esc(search) + '"</div>';
    }
    // Always show a hint if no search text
    if (!search && groups.length > 0) {
        html += '<div style="padding:6px 12px;font-size:11px;color:var(--text-muted);border-top:1px solid var(--border)">Type a new name to create a group</div>';
    }
    list.innerHTML = html;
}

function filterDesignerGroupPicker() { populateDesignerGroupList(); }

function selectDesignerGroup(group) {
    const label = document.getElementById('ds-group-picker-label');
    const select = document.getElementById('f-group');
    if (label) label.textContent = group || 'Default';
    if (select) select.value = group;
    dsGroupPickerOpen = false;
    document.getElementById('ds-group-picker-popover').style.display = 'none';
    document.removeEventListener('click', closeDesignerGroupPickerOutside);
}

async function createAndSelectDesignerGroup(name) {
    try {
        await api('POST', '/api/jobs/groups', { name: name });
        if (typeof fetchGroups === 'function') await fetchGroups();
        selectDesignerGroup(name);
        toast('Group "' + name + '" created');
    } catch (e) { toast(e.message, 'error'); }
}

function selectPipelineGroup(group) {
    const label = document.getElementById('pl-group-picker-label');
    const select = document.getElementById('stages-group-filter');
    if (label) label.textContent = group || 'All Groups';
    if (select) { select.value = group; fetchGroupsPage(); }
    plGroupPickerOpen = false;
    document.getElementById('pl-group-picker-popover').style.display = 'none';
    document.removeEventListener('click', closePipelineGroupPickerOutside);
}

function setGroupsView(mode) {
    groupsViewMode = mode;
    localStorage.setItem('kf-groupsView', mode);
    document.querySelectorAll('.groups-tab').forEach(b => {
        b.classList.toggle('active', b.id === 'gv-' + mode);
    });
    const grid = document.getElementById('groups-grid');
    const mapWrap = document.getElementById('groups-map-wrap');
    const mapFilter = document.getElementById('map-group-filter');
    const stagesFilter = document.getElementById('stages-group-filter');
    if (mode === 'map') {
        if (grid) grid.style.display = 'none';
        if (mapWrap) mapWrap.style.display = '';
        if (mapFilter) mapFilter.style.display = '';
        if (stagesFilter) stagesFilter.style.display = 'none';
        renderMap();
    } else if (mode === 'pipeline') {
        if (grid) grid.style.display = '';
        if (mapWrap) mapWrap.style.display = 'none';
        if (mapFilter) mapFilter.style.display = 'none';
        if (stagesFilter) stagesFilter.style.display = '';
        fetchGroupsPage();
    } else {
        if (grid) grid.style.display = '';
        if (mapWrap) mapWrap.style.display = 'none';
        if (mapFilter) mapFilter.style.display = 'none';
        if (stagesFilter) stagesFilter.style.display = 'none';
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

        // Restore tab state
        document.querySelectorAll('.groups-tab').forEach(b => {
            b.classList.toggle('active', b.id === 'gv-' + groupsViewMode);
        });

        // Populate stages group filter
        const stagesFilter = document.getElementById('stages-group-filter');
        if (stagesFilter) {
            const currentSel = stagesFilter.value;
            stagesFilter.innerHTML = '<option value="">All Groups</option>';
            for (const g of sortedGroups) {
                stagesFilter.innerHTML += '<option value="' + esc(g) + '"' + (g === currentSel ? ' selected' : '') + '>' + esc(g) + '</option>';
            }
            // Restore previous selection (or stay on All Groups)
            if (currentSel) stagesFilter.value = currentSel;
        }

        if (groupsViewMode === 'pipeline') {
            const selectedGroup = stagesFilter ? stagesFilter.value : '';
            // Show selected group, or all groups if none selected
            const groupsToShow = selectedGroup ? [selectedGroup] : sortedGroups.filter(g => (jobsByGroup[g] || []).length > 0);
            let scheduleMap = {};
            for (const g of groupsToShow) {
                try {
                    const sched = await api('GET', '/api/jobs/pipeline-schedule/' + encodeURIComponent(g));
                    if (sched && sched.type) scheduleMap[g] = sched;
                } catch (e) { /* no schedule */ }
            }
            if (groupsToShow.length > 0) {
                renderPipelineView(groupsToShow, jobsByGroup, scheduleMap);
            } else {
                document.getElementById('groups-grid').innerHTML = '<div style="padding:24px;text-align:center;color:var(--text-muted)">No groups with jobs found.</div>';
            }
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
        html += '<div class="group-card-header" style="cursor:pointer" onclick="navToGroupJobs(\'' + escAttr(g) + '\')">';
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
        html += '<button class="btn btn-ghost btn-sm" onclick="renameGroup(\'' + escAttr(g) + '\')">Rename</button>';
        html += '<button class="btn btn-ghost btn-sm" style="color:var(--danger)" onclick="deleteGroup(\'' + escAttr(g) + '\')">Delete</button>';
        html += '</div>';
        html += '</div>';
    }
    grid.innerHTML = html;
}

function renderPipelineView(sortedGroups, jobsByGroup, scheduleMap) {
    scheduleMap = scheduleMap || {};
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

        // Pipeline schedule badge
        const sched = scheduleMap[g];
        let schedBadge = '';
        if (sched && sched.type) {
            let schedText = '';
            if (sched.type === 'cron') schedText = 'Cron: ' + (sched.value || '');
            else if (sched.type === 'interval') schedText = 'Every ' + formatInterval((sched.value && sched.value.interval_secs) || 0);
            schedBadge = '<span style="font-size:11px;padding:2px 8px;border-radius:10px;background:var(--accent);color:#fff;white-space:nowrap" title="' + esc(schedText) + '">&#128339; ' + esc(schedText) + '</span>';
        }

        // Pipeline header
        html += '<div style="margin-bottom:20px">';
        html += '<div style="display:flex;align-items:center;gap:8px;margin-bottom:10px;cursor:pointer" onclick="navToGroupJobs(\'' + escAttr(g) + '\')">';
        html += '<span style="width:12px;height:12px;border-radius:50%;background:' + color + ';flex-shrink:0"></span>';
        html += '<strong style="font-size:14px">' + esc(g) + '</strong>';
        html += '<span style="font-size:12px;color:var(--text-muted)">' + groupJobs.length + ' stage' + (groupJobs.length !== 1 ? 's' : '') + '</span>';
        html += schedBadge;
        html += '<button class="btn btn-sm" style="margin-left:auto" onclick="event.stopPropagation();openPipelineHistory(\'' + escAttr(g) + '\')">&#128203; History</button>';
        html += '<button class="btn btn-sm" onclick="event.stopPropagation();openPipelineSchedule(\'' + escAttr(g) + '\')">&#128339; Schedule</button>';
        html += '<button class="btn btn-primary btn-sm" onclick="event.stopPropagation();triggerPipeline(\'' + escAttr(g) + '\')">&#9654; Run Pipeline</button>';
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
            // Determine if this job has ever succeeded (for showing green on pending re-runs)
            const hasSucceeded = (j.execution_counts || {}).succeeded > 0;
            let bg, border, statusIcon, label;
            if (status === 'succeeded') { bg = 'rgba(46,204,113,0.12)'; border = '#2ecc71'; statusIcon = '\u2714'; label = 'succeeded'; }
            else if (status === 'failed' || status === 'timed_out') { bg = 'rgba(224,82,82,0.12)'; border = '#e05252'; statusIcon = '\u2718'; label = status; }
            else if (status === 'running') { bg = 'rgba(62,139,255,0.12)'; border = '#3e8bff'; statusIcon = '\u25B6'; label = 'running'; }
            else if (status === 'pending') {
                // Pending re-run: show green with a re-run indicator if previously succeeded
                if (hasSucceeded) { bg = 'rgba(46,204,113,0.12)'; border = '#2ecc71'; statusIcon = '\u21BB'; label = 'queued'; }
                else { bg = 'rgba(62,139,255,0.12)'; border = '#3e8bff'; statusIcon = '\u23F3'; label = 'pending'; }
            }
            else if (status === 'pending_approval') { bg = 'rgba(230,168,23,0.12)'; border = '#e6a817'; statusIcon = '\u23F3'; label = 'awaiting approval'; }
            else if (status === 'cancelled') { bg = 'var(--bg-tertiary)'; border = 'var(--border)'; statusIcon = '\u2298'; label = 'cancelled'; }
            else if (status === 'skipped') { bg = 'var(--bg-tertiary)'; border = 'var(--border)'; statusIcon = '\u23ED'; label = 'skipped'; }
            else { bg = 'var(--bg-tertiary)'; border = 'var(--border)'; statusIcon = '\u25CB'; label = 'idle'; }
            let s = '<div style="background:' + bg + ';border:2px solid ' + border + ';border-radius:8px;padding:10px 14px;min-width:130px;cursor:pointer;flex-shrink:0;text-align:center" onclick="showJobDetail(\'' + j.id + '\')">';
            s += '<div style="font-size:18px;margin-bottom:4px">' + statusIcon + '</div>';
            s += '<div style="font-size:12px;font-weight:600;margin-bottom:2px;white-space:nowrap">' + esc(j.name) + '</div>';
            s += '<div style="font-size:10px;color:var(--text-muted)">' + label + '</div>';
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

async function triggerPipeline(group) {
    try {
        const jobsRes = await api('GET', '/api/jobs?per_page=1000&group=' + encodeURIComponent(group));
        const jobs = jobsRes.data;
        const groupIds = new Set(jobs.map(j => j.id));
        const roots = jobs.filter(j => !(j.depends_on || []).some(d => groupIds.has(d.job_id)));
        if (roots.length === 0) { toast('No root jobs found in "' + group + '"', 'error'); return; }
        if (!confirm('Run pipeline "' + group + '"? Will trigger ' + roots.length + ' root job(s), dependencies cascade automatically.')) return;
        let triggered = 0;
        for (const j of roots) {
            try { await api('POST', '/api/jobs/' + j.id + '/trigger'); triggered++; } catch (e) { console.error(e); }
        }
        toast(triggered + ' root job(s) triggered — pipeline running', 'success');
        if (typeof fetchJobs === 'function') fetchJobs();
        if (typeof renderJobsStagesTab === 'function') setTimeout(renderJobsStagesTab, 1000);
    } catch (e) {
        toast(e.message, 'error');
    }
}

function navToGroupJobs(group) {
    setGroupFilter(group);
    setJobsTab('list');
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

// --- Pipeline Schedule ---

let pipelineScheduleGroup = '';

async function openPipelineSchedule(group) {
    pipelineScheduleGroup = group;
    document.getElementById('pipeline-schedule-title').textContent = 'Pipeline Schedule — ' + group;
    document.getElementById('ps-type').value = 'none';
    document.getElementById('ps-cron').value = '';
    document.getElementById('ps-interval-val').value = '60';
    document.getElementById('ps-interval-unit').value = '60';
    document.getElementById('ps-current').style.display = 'none';
    document.getElementById('ps-remove-btn').style.display = 'none';
    updatePipelineScheduleUI();

    try {
        const sched = await api('GET', '/api/jobs/pipeline-schedule/' + encodeURIComponent(group));
        if (sched && sched.type) {
            if (sched.type === 'cron') {
                document.getElementById('ps-type').value = 'cron';
                document.getElementById('ps-cron').value = sched.value || '';
                document.getElementById('ps-current').style.display = '';
                document.getElementById('ps-current-text').textContent = 'Cron: ' + (sched.value || '');
                document.getElementById('ps-remove-btn').style.display = '';
            } else if (sched.type === 'interval') {
                document.getElementById('ps-type').value = 'interval';
                const secs = (sched.value && sched.value.interval_secs) || 3600;
                if (secs % 86400 === 0) {
                    document.getElementById('ps-interval-val').value = secs / 86400;
                    document.getElementById('ps-interval-unit').value = '86400';
                } else if (secs % 3600 === 0) {
                    document.getElementById('ps-interval-val').value = secs / 3600;
                    document.getElementById('ps-interval-unit').value = '3600';
                } else {
                    document.getElementById('ps-interval-val').value = Math.round(secs / 60);
                    document.getElementById('ps-interval-unit').value = '60';
                }
                document.getElementById('ps-current').style.display = '';
                document.getElementById('ps-current-text').textContent = 'Every ' + formatInterval(secs);
                document.getElementById('ps-remove-btn').style.display = '';
            }
            updatePipelineScheduleUI();
        }
    } catch (e) {
        // No schedule set — that's fine
    }

    openModal('pipeline-schedule-modal');
}

function updatePipelineScheduleUI() {
    const type = document.getElementById('ps-type').value;
    document.getElementById('ps-cron-section').style.display = type === 'cron' ? '' : 'none';
    document.getElementById('ps-interval-section').style.display = type === 'interval' ? '' : 'none';
}

function formatInterval(secs) {
    if (secs >= 86400 && secs % 86400 === 0) return (secs / 86400) + ' day' + (secs / 86400 !== 1 ? 's' : '');
    if (secs >= 3600 && secs % 3600 === 0) return (secs / 3600) + ' hour' + (secs / 3600 !== 1 ? 's' : '');
    if (secs >= 60) return Math.round(secs / 60) + ' minute' + (Math.round(secs / 60) !== 1 ? 's' : '');
    return secs + ' second' + (secs !== 1 ? 's' : '');
}

async function savePipelineSchedule() {
    const type = document.getElementById('ps-type').value;
    const group = pipelineScheduleGroup;

    if (type === 'none') {
        await removePipelineSchedule();
        return;
    }

    let schedule;
    if (type === 'cron') {
        const expr = document.getElementById('ps-cron').value.trim();
        if (!expr) { toast('Enter a cron expression', 'error'); return; }
        schedule = { type: 'cron', value: expr };
    } else if (type === 'interval') {
        const val = parseInt(document.getElementById('ps-interval-val').value) || 1;
        const unit = parseInt(document.getElementById('ps-interval-unit').value) || 60;
        const secs = val * unit;
        if (secs < 60) { toast('Interval must be at least 1 minute', 'error'); return; }
        schedule = { type: 'interval', value: { interval_secs: secs } };
    }

    try {
        await api('PUT', '/api/jobs/pipeline-schedule/' + encodeURIComponent(group), { schedule });
        toast('Pipeline schedule saved for "' + group + '"', 'success');
        closeModal('pipeline-schedule-modal');
        fetchGroupsPage();
    } catch (e) {
        toast('Error: ' + e.message, 'error');
    }
}

async function removePipelineSchedule() {
    const group = pipelineScheduleGroup;
    try {
        await api('DELETE', '/api/jobs/pipeline-schedule/' + encodeURIComponent(group));
        toast('Pipeline schedule removed for "' + group + '"', 'success');
        closeModal('pipeline-schedule-modal');
        fetchGroupsPage();
    } catch (e) {
        toast('Error: ' + e.message, 'error');
    }
}

// --- Pipeline Run History ---

async function openPipelineHistory(group) {
    document.getElementById('pipeline-history-title').textContent = 'Run History — ' + group;
    document.getElementById('pipeline-history-content').innerHTML = '<div style="text-align:center;color:var(--text-muted);padding:24px">Loading...</div>';
    openModal('pipeline-history-modal');

    try {
        // Fetch jobs in this group to build a name map
        const jobsRes = await api('GET', '/api/jobs?per_page=1000&group=' + encodeURIComponent(group));
        const jobs = jobsRes.data;
        const jobMap = {};
        for (const j of jobs) jobMap[j.id] = j;
        const jobIds = new Set(jobs.map(j => j.id));

        // Fetch recent executions for this group
        const execsRes = await api('GET', '/api/executions?per_page=200&group=' + encodeURIComponent(group));
        const execs = execsRes.data.filter(e => jobIds.has(e.job_id));

        if (execs.length === 0) {
            document.getElementById('pipeline-history-content').innerHTML =
                '<div style="text-align:center;color:var(--text-muted);padding:24px">No executions found for this pipeline.</div>';
            return;
        }

        // Cluster executions into pipeline "runs" by time proximity
        // Sort by started_at descending
        execs.sort((a, b) => new Date(b.started_at || b.finished_at || 0) - new Date(a.started_at || a.finished_at || 0));

        const runs = clusterPipelineRuns(execs, jobs);
        renderPipelineHistory(runs, jobMap, jobs);
    } catch (e) {
        document.getElementById('pipeline-history-content').innerHTML =
            '<div style="text-align:center;color:var(--danger);padding:24px">Error loading history: ' + esc(e.message) + '</div>';
    }
}

function clusterPipelineRuns(execs, jobs) {
    // Group executions into runs: a run starts when a root job fires, and includes
    // all executions that started within a time window (5 minutes per stage depth)
    const inGroup = new Set(jobs.map(j => j.id));
    const rootIds = new Set(jobs.filter(j => !(j.depends_on || []).some(d => inGroup.has(d.job_id))).map(j => j.id));
    const maxWindow = Math.max(jobs.length * 300000, 600000); // 5 min per job, min 10 min

    const runs = [];
    const used = new Set();

    for (const exec of execs) {
        if (used.has(exec.id)) continue;
        // Start a new run from this execution
        const runStart = new Date(exec.started_at || exec.finished_at || Date.now());
        const run = { started: runStart, executions: [] };

        // Collect all executions within the time window
        for (const e of execs) {
            if (used.has(e.id)) continue;
            const t = new Date(e.started_at || e.finished_at || Date.now());
            if (Math.abs(t - runStart) <= maxWindow) {
                run.executions.push(e);
                used.add(e.id);
            }
        }

        runs.push(run);
    }

    return runs;
}

function renderPipelineHistory(runs, jobMap, jobs) {
    const container = document.getElementById('pipeline-history-content');

    // Topological sort for consistent column order
    const inGroup = new Set(jobs.map(j => j.id));
    const depMap = {};
    for (const j of jobs) depMap[j.id] = (j.depends_on || []).filter(d => inGroup.has(d.job_id)).map(d => d.job_id);
    const sorted = [];
    const visited = new Set();
    function visit(id) {
        if (visited.has(id)) return;
        visited.add(id);
        for (const pid of (depMap[id] || [])) visit(pid);
        sorted.push(id);
    }
    for (const j of jobs) visit(j.id);

    let html = '<table style="width:100%;border-collapse:collapse;font-size:12px">';
    // Header
    html += '<thead><tr>';
    html += '<th style="padding:6px 8px;text-align:left;border-bottom:2px solid var(--border);white-space:nowrap">Run</th>';
    html += '<th style="padding:6px 8px;text-align:left;border-bottom:2px solid var(--border);white-space:nowrap">Status</th>';
    for (const id of sorted) {
        const j = jobMap[id];
        if (!j) continue;
        html += '<th style="padding:6px 8px;text-align:center;border-bottom:2px solid var(--border);white-space:nowrap;max-width:100px;overflow:hidden;text-overflow:ellipsis" title="' + esc(j.name) + '">' + esc(j.name) + '</th>';
    }
    html += '<th style="padding:6px 8px;text-align:right;border-bottom:2px solid var(--border);white-space:nowrap">Duration</th>';
    html += '</tr></thead><tbody>';

    for (let i = 0; i < runs.length; i++) {
        const run = runs[i];
        const execByJob = {};
        for (const e of run.executions) {
            // Keep latest execution per job within this run
            if (!execByJob[e.job_id] || new Date(e.started_at || 0) > new Date(execByJob[e.job_id].started_at || 0)) {
                execByJob[e.job_id] = e;
            }
        }

        // Overall run status
        const allExecs = Object.values(execByJob);
        const hasRunning = allExecs.some(e => e.status === 'running' || e.status === 'pending');
        const hasFailed = allExecs.some(e => e.status === 'failed' || e.status === 'timed_out');
        const allSucceeded = allExecs.length === sorted.length && allExecs.every(e => e.status === 'succeeded');
        let overallStatus, overallColor;
        if (hasRunning) { overallStatus = 'running'; overallColor = '#3e8bff'; }
        else if (allSucceeded) { overallStatus = 'succeeded'; overallColor = '#2ecc71'; }
        else if (hasFailed) { overallStatus = 'failed'; overallColor = '#e05252'; }
        else { overallStatus = 'partial'; overallColor = '#e6a817'; }

        // Duration: from earliest start to latest finish
        let earliest = null, latest = null;
        for (const e of allExecs) {
            const s = e.started_at ? new Date(e.started_at) : null;
            const f = e.finished_at ? new Date(e.finished_at) : null;
            if (s && (!earliest || s < earliest)) earliest = s;
            if (f && (!latest || f > latest)) latest = f;
        }
        let duration = '';
        if (earliest && latest) {
            const secs = Math.round((latest - earliest) / 1000);
            if (secs >= 3600) duration = Math.floor(secs / 3600) + 'h ' + Math.floor((secs % 3600) / 60) + 'm';
            else if (secs >= 60) duration = Math.floor(secs / 60) + 'm ' + (secs % 60) + 's';
            else duration = secs + 's';
        } else if (hasRunning) {
            duration = 'running...';
        }

        html += '<tr style="border-bottom:1px solid var(--border)">';
        html += '<td style="padding:6px 8px;white-space:nowrap">' + fmtDate(run.started) + '</td>';
        html += '<td style="padding:6px 8px"><span style="display:inline-block;padding:2px 8px;border-radius:10px;font-size:11px;font-weight:600;color:#fff;background:' + overallColor + '">' + overallStatus + '</span></td>';

        for (const id of sorted) {
            const e = execByJob[id];
            html += '<td style="padding:6px 8px;text-align:center">';
            if (e) {
                let icon, color, title;
                if (e.status === 'succeeded') { icon = '\u2714'; color = '#2ecc71'; title = 'succeeded'; }
                else if (e.status === 'failed' || e.status === 'timed_out') { icon = '\u2718'; color = '#e05252'; title = e.status; }
                else if (e.status === 'running') { icon = '\u25B6'; color = '#3e8bff'; title = 'running'; }
                else if (e.status === 'pending') { icon = '\u23F3'; color = '#3e8bff'; title = 'pending'; }
                else if (e.status === 'pending_approval') { icon = '\u23F3'; color = '#e6a817'; title = 'awaiting approval'; }
                else if (e.status === 'cancelled') { icon = '\u2298'; color = 'var(--text-muted)'; title = 'cancelled'; }
                else if (e.status === 'skipped') { icon = '\u23ED'; color = 'var(--text-muted)'; title = 'skipped'; }
                else { icon = '\u25CB'; color = 'var(--text-muted)'; title = e.status; }
                html += '<span style="cursor:pointer;color:' + color + ';font-size:16px" title="' + esc(title) + '" onclick="showExecDetail(\'' + e.id + '\')">' + icon + '</span>';
            } else {
                html += '<span style="color:var(--text-muted)" title="not executed">&mdash;</span>';
            }
            html += '</td>';
        }

        html += '<td style="padding:6px 8px;text-align:right;white-space:nowrap;color:var(--text-muted)">' + duration + '</td>';
        html += '</tr>';
    }

    html += '</tbody></table>';

    if (runs.length === 0) {
        html = '<div style="text-align:center;color:var(--text-muted);padding:24px">No pipeline runs found.</div>';
    }

    container.innerHTML = html;
}
