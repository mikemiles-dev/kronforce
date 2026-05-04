// Kronforce - Job CRUD, list, detail, actions, bulk selection, templates
// --- Job Groups ---
let cachedGroups = [];
let groupFilter = localStorage.getItem('kf-group-filter') || '';
let jobsTab = 'list';

function setJobsTab(tab) {
    jobsTab = tab;
    document.querySelectorAll('.jobs-tab').forEach(b => {
        b.classList.toggle('active', b.id === 'jt-' + tab);
    });
    const panels = { list: 'jobs-list-panel', stages: 'jobs-stages-panel', map: 'jobs-map-panel' };
    for (const [key, id] of Object.entries(panels)) {
        const el = document.getElementById(id);
        if (el) el.style.display = key === tab ? '' : 'none';
    }

    if (tab === 'list') {
        fetchJobs();
    } else if (tab === 'stages') {
        renderJobsStagesTab();
    } else if (tab === 'map') {
        renderMap();
    }
    if (typeof updateHash === 'function') updateHash();
}

function applyJobFilters(jobs) {
    let filtered = jobs;
    const filter = jobSearch.statusFilter;
    const search = jobSearch.searchTerm;
    if (filter === 'blocked') {
        filtered = filtered.filter(j => (j.depends_on || []).length > 0 && !j.deps_satisfied);
    } else if (filter === 'running') {
        filtered = filtered.filter(j => j.last_execution && j.last_execution.status === 'running');
    } else if (filter === 'failed') {
        filtered = filtered.filter(j => (j.execution_counts && j.execution_counts.failed > 0) || (j.last_execution && (j.last_execution.status === 'failed' || j.last_execution.status === 'timed_out')));
    } else if (filter === 'scheduled') {
        filtered = filtered.filter(j => j.status === 'scheduled');
    } else if (filter === 'paused') {
        filtered = filtered.filter(j => j.status === 'paused');
    } else if (filter === 'unscheduled') {
        filtered = filtered.filter(j => j.status === 'unscheduled' && !((j.depends_on || []).length > 0 && !j.deps_satisfied));
    }
    if (search) {
        filtered = filtered.filter(j => j.name.toLowerCase().includes(search) || (j.description && j.description.toLowerCase().includes(search)));
    }
    if (groupFilter) {
        filtered = filtered.filter(j => (j.group || 'Default') === groupFilter);
    }
    return filtered;
}

async function renderJobsStagesTab() {
    try {
        const [groups, jobsRes] = await Promise.all([
            api('GET', '/api/jobs/groups'),
            api('GET', '/api/jobs?per_page=1000'),
        ]);
        const jobs = applyJobFilters(jobsRes.data);
        const jobsByGroup = {};
        for (const j of jobs) {
            const g = j.group || 'Default';
            if (!jobsByGroup[g]) jobsByGroup[g] = [];
            jobsByGroup[g].push(j);
        }

        const content = document.getElementById('stages-content');
        if (!content) return;

        const allGroups = new Set(groups);
        for (const g of Object.keys(jobsByGroup)) allGroups.add(g);
        const sortedGroups = [...allGroups].sort((a, b) => {
            if (a === 'Default') return -1;
            if (b === 'Default') return 1;
            return a.localeCompare(b);
        });

        // Group filter already applied in applyJobFilters, show all groups that have jobs
        const groupsToShow = sortedGroups;

        // Temporarily set ID so renderPipelineView writes to stages-content
        content.id = 'groups-grid';
        renderPipelineView(groupsToShow.filter(g => jobsByGroup[g] && jobsByGroup[g].length > 0), jobsByGroup);
        content.id = 'stages-content';

        if (jobs.length === 0) {
            content.innerHTML = '<div style="padding:24px;text-align:center;color:var(--text-muted)">No jobs match the current filters.</div>';
        }
    } catch (e) {
        console.error('renderJobsStagesTab:', e);
    }
}

const GROUP_COLORS = ['var(--accent)', 'var(--success)', 'var(--warning)', 'var(--danger)', 'var(--info)', '#9b59b6', '#1abc9c', '#e67e22'];

function groupColor(name) {
    let hash = 0;
    for (let i = 0; i < name.length; i++) hash = ((hash << 5) - hash + name.charCodeAt(i)) | 0;
    return GROUP_COLORS[Math.abs(hash) % GROUP_COLORS.length];
}

function groupBadge(group) {
    if (!group) return '';
    return ' <span class="group-badge" style="background:' + groupColor(group) + '">' + esc(group) + '</span>';
}

async function fetchGroups() {
    try {
        cachedGroups = await api('GET', '/api/jobs/groups');
        renderGroupPickerList();
    } catch (e) {
        console.error('fetchGroups:', e);
    }
}

function renderGroupPickerList(filter) {
    const list = document.getElementById('group-picker-list');
    if (!list) return;
    const term = (filter || '').toLowerCase();
    let groups = cachedGroups.filter(g => !term || g.toLowerCase().includes(term));
    let html = '<div class="group-picker-item group-picker-create" onclick="createNewGroupFromPicker()">+ New Group</div>';
    html += '<div class="group-picker-item' + (!groupFilter ? ' active' : '') + '" onclick="setGroupFilter(\'\')">All Groups</div>';
    for (const g of groups) {
        const color = groupColor(g);
        const isActive = groupFilter === g;
        const canDelete = g !== 'Default';
        html += '<div class="group-picker-item' + (isActive ? ' active' : '') + '" onclick="setGroupFilter(\'' + esc(g).replace(/'/g, "\\'") + '\')">';
        html += '<span class="group-picker-dot" style="background:' + color + '"></span>';
        html += '<span style="flex:1">' + esc(g) + '</span>';
        if (canDelete) {
            html += '<button class="group-picker-delete" onclick="event.stopPropagation();deleteGroupFromPicker(\'' + esc(g).replace(/'/g, "\\'") + '\')" title="Delete group">&times;</button>';
        }
        html += '</div>';
    }
    if (groups.length === 0 && term) {
        html += '<div class="group-picker-empty">No groups match</div>';
    }
    list.innerHTML = html;
}

async function createNewGroupFromPicker() {
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
        await fetchGroups();
        renderGroupPickerList();
    } catch (e) {
        toast(e.message, 'error');
    }
}

async function deleteGroupFromPicker(name) {
    if (!confirm('Delete group "' + name + '"? Jobs will move to Default.')) return;
    try {
        await api('PUT', '/api/jobs/rename-group', { old_name: name, new_name: 'Default' });
        toast('Group "' + name + '" deleted');
        if (groupFilter === name) setGroupFilter('');
        await fetchGroups();
        renderGroupPickerList();
    } catch (e) {
        toast(e.message, 'error');
    }
}

function toggleGroupPicker() {
    const pop = document.getElementById('group-picker-popover');
    if (!pop) return;
    const showing = pop.style.display !== 'none';
    pop.style.display = showing ? 'none' : '';
    if (!showing) {
        // Position fixed popover below the button
        const btn = document.getElementById('group-picker-btn');
        if (btn) {
            const rect = btn.getBoundingClientRect();
            pop.style.top = (rect.bottom + 4) + 'px';
            pop.style.left = rect.left + 'px';
        }
        const search = document.getElementById('group-picker-search');
        if (search) { search.value = ''; search.focus(); }
        renderGroupPickerList();
    }
}

function filterGroupPicker() {
    const search = document.getElementById('group-picker-search');
    renderGroupPickerList(search ? search.value : '');
}

async function triggerGroup() {
    if (!groupFilter) { toast('Select a group first', 'error'); return; }
    const jobs = allJobs.filter(j => (j.group || 'Default') === groupFilter);
    // Find root jobs: jobs in this group with no dependencies on other jobs in this group
    const groupIds = new Set(jobs.map(j => j.id));
    const roots = jobs.filter(j => !(j.depends_on || []).some(d => groupIds.has(d.job_id)));
    if (roots.length === 0) { toast('No root jobs found in group', 'error'); return; }
    if (!confirm('Trigger ' + roots.length + ' root job(s) in "' + groupFilter + '"? Dependent jobs will cascade automatically.')) return;
    let triggered = 0;
    for (const j of roots) {
        try {
            await api('POST', '/api/jobs/' + j.id + '/trigger');
            triggered++;
        } catch (e) { console.error('trigger failed for ' + j.name, e); }
    }
    toast(triggered + ' root job(s) triggered — dependencies will cascade', 'success');
    fetchJobs();
}

function setGroupFilter(value) {
    groupFilter = value;
    // Persist across refresh
    if (value) localStorage.setItem('kf-group-filter', value);
    else localStorage.removeItem('kf-group-filter');
    // Update button label
    const label = document.getElementById('group-picker-label');
    if (label) label.textContent = value || 'All Groups';
    // Highlight active state on button
    const btn = document.getElementById('group-picker-btn');
    if (btn) btn.classList.toggle('group-picker-active', !!value);
    // Close picker
    const pop = document.getElementById('group-picker-popover');
    if (pop) pop.style.display = 'none';
    // Show/hide Run Group button
    var rgb = document.getElementById('run-group-btn');
    if (rgb) rgb.style.display = value ? '' : 'none';
    // Re-render current tab
    setJobsTab(jobsTab);
    // Update URL hash
    if (typeof updateHash === 'function') updateHash();
}

// Close group picker when clicking outside
document.addEventListener('mousedown', function(e) {
    const wrap = document.getElementById('group-picker-wrap');
    if (wrap && !wrap.contains(e.target)) {
        const pop = document.getElementById('group-picker-popover');
        if (pop) pop.style.display = 'none';
    }
});

async function bulkSetGroup() {
    if (selectedJobs.size === 0) return;
    let msg = 'Select a group for ' + selectedJobs.size + ' job(s):\n\n';
    msg += '0 - Remove from group (ungrouped)\n';
    for (let i = 0; i < cachedGroups.length; i++) {
        msg += (i + 1) + ' - ' + cachedGroups[i] + '\n';
    }
    msg += '\nEnter a number, or type a new group name:';
    const input = prompt(msg);
    if (input === null) return;
    let group = null;
    const num = parseInt(input);
    if (input.trim() === '0' || input.trim() === '') {
        group = null;
    } else if (!isNaN(num) && num >= 1 && num <= cachedGroups.length) {
        group = cachedGroups[num - 1];
    } else {
        group = input.trim();
    }
    try {
        await api('PUT', '/api/jobs/bulk-group', { job_ids: [...selectedJobs], group });
        selectedJobs.clear();
        fetchGroups();
        fetchJobs();
        toast(group ? 'Jobs moved to "' + group + '"' : 'Jobs ungrouped');
    } catch (e) {
        toast('Error: ' + e.message, 'error');
    }
}

// --- Jobs List ---
async function fetchJobs(resetPage) {
    if (resetPage) jobsPage = 1;
    try {
        const filter = jobSearch.statusFilter;
        const isClientFilter = filter === 'blocked' || filter === 'running' || filter === 'failed';
        const search = jobSearch.searchTerm;
        // "scheduled" filter should include waiting (scheduled) jobs, so send scheduled to API
        // "blocked", "running", "failed" are client-side filters on last_execution status
        const apiFilter = filter === 'scheduled' ? 'scheduled' : (isClientFilter ? '' : filter);
        let qs = '?page=' + jobsPage + '&per_page=' + (isClientFilter ? 100 : PER_PAGE);
        if (apiFilter) qs += '&status=' + apiFilter;
        if (search) qs += '&search=' + encodeURIComponent(search);
        if (groupFilter) qs += '&group=' + encodeURIComponent(groupFilter);
        const res = await api('GET', '/api/jobs' + qs);
        // Apply client-side filters (blocked, running, failed use applyJobFilters;
        // scheduled/paused already filtered server-side via apiFilter)
        if (isClientFilter) {
            allJobs = applyJobFilters(res.data);
        } else {
            allJobs = res.data;
            // Unscheduled: exclude waiting jobs (they're technically scheduled)
            if (filter === 'unscheduled') {
                allJobs = allJobs.filter(j => !((j.depends_on || []).length > 0 && !j.deps_satisfied));
            }
        }
        const isTimeFiltered = false;
        jobsTotalPages = (isClientFilter || isTimeFiltered) ? 1 : res.total_pages;
        jobsTotal = (isClientFilter || isTimeFiltered) ? allJobs.length : res.total;
        renderJobsTable();
        renderPagination('jobs-pagination', jobsPage, jobsTotalPages, jobsTotal, goToJobsPage);
    } catch (e) {
        console.error('fetchJobs:', e);
    }
}

function goToJobsPage(p) {
    jobsPage = p;
    fetchJobs();
}

function sortJobs(col) {
    if (sortColumn === col) {
        sortDirection = sortDirection === 'asc' ? 'desc' : 'asc';
    } else {
        sortColumn = col;
        sortDirection = 'asc';
    }
    renderJobsTable();
}

function getSortValue(job, col) {
    switch (col) {
        case 'name': return job.name.toLowerCase();
        case 'state': return job.status;
        case 'target':
            if (!job.target) return 'controller';
            return job.target.type;
        case 'last_run':
            if (!job.last_execution) return '';
            return job.last_execution.status;
        case 'runs': return job.execution_counts.total;
        case 'schedule': return job.schedule.type;
        case 'next_fire':
            return job.next_fire_time || '';
        default: return '';
    }
}

function renderJobsTable() {
    const wrap = document.getElementById('jobs-table-wrap');
    if (allJobs.length === 0) {
        const hasFilters = jobSearch.statusFilter || jobSearch.searchTerm || groupFilter || timeRanges.jobs;
        if (hasFilters) {
            wrap.innerHTML = renderRichEmptyState({
                icon: '&#128270;',
                title: 'No matching jobs',
                description: 'No jobs match the current filters. Try adjusting your search or filter criteria.',
                actions: [
                    { label: 'Clear Filters', onclick: "jobSearch.setStatusFilter(document.querySelector('#status-filters .status-btn'), '');fetchJobs(true)", primary: true },
                ],
            });
        } else {
            wrap.innerHTML = renderRichEmptyState({
                icon: '&#128203;',
                title: 'No jobs yet',
                description: 'Jobs are automated tasks that run on a schedule, on events, or on demand. Pick a template to get started.',
                actions: [
                    { label: 'Health Check', onclick: "openTemplateJob('health-check')", primary: true },
                    { label: 'Cron Task', onclick: "openTemplateJob('cron-task')" },
                    { label: 'Event Watcher', onclick: "openTemplateJob('event-watcher')" },
                    { label: 'Create from scratch', onclick: 'openCreateModal()' },
                ],
            });
        }
        return;
    }

    // Sort
    const sorted = [...allJobs].sort((a, b) => {
        let va = getSortValue(a, sortColumn);
        let vb = getSortValue(b, sortColumn);
        if (typeof va === 'number' && typeof vb === 'number') {
            return sortDirection === 'asc' ? va - vb : vb - va;
        }
        va = String(va); vb = String(vb);
        const cmp = va.localeCompare(vb);
        return sortDirection === 'asc' ? cmp : -cmp;
    });

    function sortTh(col, label) {
        const cls = sortColumn === col ? (sortDirection === 'asc' ? 'sortable sort-asc' : 'sortable sort-desc') : 'sortable';
        return '<th class="' + cls + '" onclick="sortJobs(\'' + col + '\')">' + label + '</th>';
    }

    let html = '<table><thead><tr><th><input type="checkbox" class="job-checkbox" onchange="toggleSelectAll(this)"></th>';
    html += sortTh('name', 'Name');
    html += sortTh('state', 'State');
    html += sortTh('target', 'Target');
    html += sortTh('last_run', 'Last Run');
    html += sortTh('runs', 'Runs');
    html += sortTh('schedule', 'Schedule');
    html += sortTh('next_fire', 'Next Fire');
    html += '<th>Actions</th></tr></thead><tbody>';
    for (const j of sorted) {
        const rowClass = j.last_execution ? 'run-' + j.last_execution.status : '';
        const checked = selectedJobs.has(j.id) ? ' checked' : '';
        html += '<tr class="' + rowClass + '">';
        html += '<td><input type="checkbox" class="job-checkbox" data-id="' + j.id + '" onchange="toggleSelectJob(this)"' + checked + '></td>';
        const approvalBadge = j.approval_required ? ' <span class="badge badge-pending_approval" style="font-size:9px;padding:1px 4px" title="Requires approval">approval</span>' : '';
        html += '<td><span class="job-name" onmousedown="this._md=Date.now()" onclick="if(window.getSelection().toString()||Date.now()-this._md>300)return;showJobDetail(\'' + j.id + '\')">' + esc(j.name) + '</span>' + approvalBadge + groupBadge(j.group) + ' ' + fmtTaskBadge(j.task) + '</td>';
        const isBlocked = (j.depends_on || []).length > 0 && !j.deps_satisfied;
        const execState = j.last_execution && (j.last_execution.status === 'running' || j.last_execution.status === 'pending_approval');
        if (isBlocked) {
            html += '<td><span class="badge badge-paused" style="cursor:pointer" onclick="showWaitingDetail(\'' + j.id + '\')" title="Click to see what this job is waiting for">waiting</span></td>';
        } else if (execState) {
            html += '<td>' + badge(j.last_execution.status) + '</td>';
        } else {
            html += '<td>' + badge(j.status) + '</td>';
        }
        html += '<td>' + fmtTarget(j.target) + '</td>';
        html += '<td>' + fmtLastRun(j.last_execution) + '</td>';
        html += '<td>' + fmtCounts(j.execution_counts, j.id) + '</td>';
        html += '<td><span class="schedule-text">' + fmtSchedule(j.schedule) + '</span></td>';
        html += '<td><span class="time-text">' + (j.next_fire_time ? fmtDate(j.next_fire_time) : '-') + '</span></td>';
        html += '<td><div class="actions">';
        if (canWrite()) {
            html += '<button class="btn-icon" title="Edit" onclick="openEditModal(\'' + j.id + '\')">&#9998;</button>';
            html += '<button class="btn-icon trigger" title="Trigger" id="trigger-' + j.id + '" onclick="triggerJob(\'' + j.id + '\')">&#9654;</button>';
            if (j.last_execution && j.last_execution.status === 'running') {
                html += '<button class="btn-icon danger" title="Stop" onclick="cancelLatestExecution(\'' + j.id + '\')">&#9632;</button>';
            }
            if (j.status === 'scheduled') {
                html += '<button class="btn-icon" title="Pause" onclick="togglePause(\'' + j.id + '\',\'scheduled\')">&#10074;&#10074;</button>';
            } else if (j.status === 'paused') {
                html += '<button class="btn-icon trigger" title="Resume" onclick="togglePause(\'' + j.id + '\',\'paused\')">&#9654;</button>';
            }
            html += '<button class="btn-icon danger" title="Delete" onclick="deleteJob(\'' + j.id + '\',\'' + esc(j.name) + '\')">&#128465;</button>';
        }
        html += '</div></td>';
        html += '</tr>';
    }
    html += '</tbody></table>';
    wrap.innerHTML = html;
    updateBulkBar();
}

// --- Job Detail ---
let detailReturnTo = 'monitor';

function setDetailTab(tab) {
    document.querySelectorAll('#detail-view .groups-tab').forEach(b => {
        b.classList.toggle('active', b.id === 'dt-' + tab);
    });
    const panels = { overview: 'detail-overview-panel', history: 'detail-history-panel', map: 'detail-map-panel' };
    for (const [key, id] of Object.entries(panels)) {
        const el = document.getElementById(id);
        if (el) el.style.display = key === tab ? '' : 'none';
    }
    // Re-render mini map when switching to map tab (Cytoscape needs visible container)
    if (tab === 'map' && currentJobId) {
        const job = allJobs.find(j => j.id === currentJobId);
        if (job) renderMiniMap(job);
    }
}

async function showJobDetail(id) {
    detailReturnTo = currentPage;
    currentJobId = id;
    execsPage = 1;
    setDetailTab('overview');
    for (const v of ALL_VIEWS) {
        document.getElementById(v + '-view').style.display = v === 'detail' ? '' : 'none';
    }
    for (const barId of Object.values(VIEW_ACTION_BARS)) {
        document.getElementById(barId).style.display = 'none';
    }

    // Update back button text
    const backLink = document.querySelector('.back-link');
    if (detailReturnTo === 'map') {
        backLink.innerHTML = '&larr; Back to Map';
    } else {
        backLink.innerHTML = '&larr; Back to Jobs';
    }

    try {
        const job = await api('GET', '/api/jobs/' + id);
        renderJobDetail(job);
        renderMiniMap(job);
        fetchJobTimeline(id);
        fetchJobEvents(id);
        await fetchExecutions(id, true);

        // Show live output if last execution is running
        var liveCard = document.getElementById('job-live-output-card');
        if (liveCard) {
            if (job.last_execution && job.last_execution.status === 'running') {
                liveCard.style.display = '';
                startJobLiveStream(job.last_execution.id);
            } else {
                liveCard.style.display = 'none';
                stopJobLiveStream();
            }
        }
    } catch (e) {
        toast(e.message, 'error');
    }
}

var jobLiveEventSource = null;
var jobLiveRetries = 0;

function startJobLiveStream(execId) {
    stopJobLiveStream();
    var pre = document.getElementById('job-live-output');
    if (!pre) return;
    pre.textContent = '';

    var streamUrl = '/api/executions/' + execId + '/stream';
    var apiKey = localStorage.getItem('kronforce-api-key');
    if (apiKey) streamUrl += '?token=' + encodeURIComponent(apiKey);
    jobLiveEventSource = new EventSource(streamUrl);
    var connected = false;
    jobLiveEventSource.onopen = function() { connected = true; jobLiveRetries = 0; };
    jobLiveEventSource.onmessage = function(event) {
        connected = true;
        pre.textContent += event.data + '\n';
        pre.scrollTop = pre.scrollHeight;
    };
    jobLiveEventSource.addEventListener('done', function() {
        stopJobLiveStream();
        if (currentJobId) showJobDetail(currentJobId);
    });
    jobLiveEventSource.onerror = function() {
        stopJobLiveStream();
        if (!connected && jobLiveRetries < 5) {
            jobLiveRetries++;
            setTimeout(function() { startJobLiveStream(execId); }, 500 * jobLiveRetries);
        } else if (connected && currentJobId) {
            setTimeout(function() { showJobDetail(currentJobId); }, 500);
        }
    };
}

function stopJobLiveStream() {
    if (jobLiveEventSource) {
        jobLiveEventSource.close();
        jobLiveEventSource = null;
    }
}

async function fetchJobEvents(jobId) {
    const wrap = document.getElementById('job-events-list');
    if (!wrap) return;
    try {
        const res = await api('GET', '/api/events?per_page=100');
        const events = res.data.filter(e => e.job_id === jobId).slice(0, 15);
        if (events.length === 0) {
            wrap.innerHTML = '<p style="color:var(--text-muted);font-size:12px;padding:8px 0">No events for this job yet.</p>';
            return;
        }
        let html = '<div style="display:flex;flex-direction:column;gap:4px;padding-top:8px">';
        for (const e of events) {
            const sevColor = e.severity === 'error' ? 'var(--danger)' : e.severity === 'warning' ? 'var(--warning)' : e.severity === 'success' ? 'var(--success)' : 'var(--text-muted)';
            const isClickable = !!e.execution_id;
            const baseStyle = 'display:flex;align-items:center;gap:8px;font-size:12px;padding:4px 6px;border-bottom:1px solid var(--border);border-radius:3px';
            const cls = isClickable ? 'job-event-row clickable' : 'job-event-row';
            const clickAttrs = isClickable ? ' style="' + baseStyle + ';cursor:pointer" onclick="showExecDetail(\'' + e.execution_id + '\')" title="View execution output"' : ' style="' + baseStyle + '"';
            html += '<div class="' + cls + '"' + clickAttrs + '>';
            html += '<span style="width:6px;height:6px;border-radius:50%;background:' + sevColor + ';flex-shrink:0"></span>';
            html += '<span style="color:var(--text-secondary);white-space:nowrap;font-size:11px">' + fmtDate(e.timestamp) + '</span>';
            html += '<span style="color:var(--accent);font-size:11px;white-space:nowrap">' + esc(e.kind) + '</span>';
            html += '<span style="flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">' + esc(e.message) + '</span>';
            if (isClickable) html += '<span class="job-event-link-icon" style="font-size:11px">&#128196; output &rsaquo;</span>';
            html += '</div>';
        }
        html += '</div>';
        wrap.innerHTML = html;
    } catch (e) {
        wrap.innerHTML = '<p style="color:var(--text-muted);font-size:12px">Failed to load events.</p>';
    }
}

function showJobsList() {
    stopJobLiveStream();
    currentJobId = null;
    document.getElementById('detail-view').style.display = 'none';
    showPage(detailReturnTo);
}

function renderJobDetail(job) {
    let deps = 'None';
    if (job.deps_status && job.deps_status.length > 0) {
        deps = job.deps_status.map(d => {
            const name = d.job_name || d.job_id.slice(0, 8);
            const icon = d.satisfied ? '<span style="color:var(--success)">\u2714</span>' : '<span style="color:var(--danger)">\u2718</span>';
            const window = d.within_secs ? ' <span class="time-text">(within ' + fmtSeconds(d.within_secs) + ')</span>' : '';
            const badgeCls = d.satisfied ? 'badge-succeeded' : 'badge-failed';
            return icon + ' <span class="badge ' + badgeCls + '" style="cursor:pointer" onclick="showJobDetail(\'' + d.job_id + '\')">' + esc(name) + '</span>' + window;
        }).join(' &nbsp; ');
        if (!job.deps_satisfied) {
            deps = '<span class="badge badge-paused" style="margin-right:8px">waiting</span>' + deps;
        }
    } else if (job.depends_on && job.depends_on.length > 0) {
        deps = job.depends_on.map(d => {
            const dj = allJobs.find(j => j.id === d.job_id);
            const name = dj ? esc(dj.name) : d.job_id.slice(0, 8);
            const window = d.within_secs ? ' <span class="time-text">(within ' + fmtSeconds(d.within_secs) + ')</span>' : '';
            return '<span class="badge badge-running">' + name + '</span>' + window;
        }).join(' ');
    }

    // Execution stats
    const counts = job.execution_counts || {};
    const statsHtml = '<span style="color:var(--success)">' + (counts.succeeded || 0) + ' passed</span> / ' +
        '<span style="color:var(--danger)">' + (counts.failed || 0) + ' failed</span> / ' +
        (counts.total || 0) + ' total';

    // Last execution (clickable to execution detail)
    const last = job.last_execution;
    const lastHtml = last
        ? '<span style="cursor:pointer" onclick="showExecDetail(\'' + last.id + '\')" title="View execution">' + badge(last.status) + (last.finished_at ? ' ' + fmtDate(last.finished_at) : '') + '</span>'
        : '<span style="color:var(--text-muted)">never run</span>';

    // Retry config
    let retryHtml = 'None';
    if (job.retry_max > 0) {
        retryHtml = job.retry_max + ' retries';
        if (job.retry_delay_secs) retryHtml += ', ' + job.retry_delay_secs + 's delay';
        if (job.retry_backoff > 1) retryHtml += ', ' + job.retry_backoff + 'x backoff';
    }

    // Notifications
    const notif = job.notifications;
    let notifHtml = '<span style="color:var(--text-muted)">off</span>';
    if (notif) {
        const triggers = [];
        if (notif.on_failure) triggers.push('failure');
        if (notif.on_success) triggers.push('success');
        if (notif.on_assertion_failure) triggers.push('assertion');
        notifHtml = triggers.length ? triggers.join(', ') : '<span style="color:var(--text-muted)">off</span>';
    }

    // Output rules summary
    const rules = job.output_rules;
    let rulesHtml = '<span style="color:var(--text-muted)">none</span>';
    if (rules) {
        const parts = [];
        if (rules.extractions && rules.extractions.length) parts.push(rules.extractions.length + ' extraction' + (rules.extractions.length > 1 ? 's' : ''));
        if (rules.triggers && rules.triggers.length) parts.push(rules.triggers.length + ' trigger' + (rules.triggers.length > 1 ? 's' : ''));
        if (rules.assertions && rules.assertions.length) parts.push(rules.assertions.length + ' assertion' + (rules.assertions.length > 1 ? 's' : ''));
        if (parts.length) rulesHtml = parts.join(', ');
    }

    // Only show non-default fields in the extras section
    const extras = [];
    if (job.retry_max > 0) extras.push(field('Retry', retryHtml));
    if (job.timeout_secs) extras.push(field('Timeout', job.timeout_secs + 's'));
    if (job.run_as) extras.push(field('Run As', '<code>' + esc(job.run_as) + '</code>'));
    if (job.priority) extras.push(field('Priority', String(job.priority)));
    if (job.max_concurrent > 0) extras.push(field('Concurrency', 'max ' + job.max_concurrent));
    if (job.parameters && job.parameters.length > 0) extras.push(field('Parameters', job.parameters.map(p => '<code>' + esc(p.name) + '</code>' + (p.required ? ' <span style="color:var(--danger)">*</span>' : '')).join(', ')));
    if (job.webhook_url) {
        extras.push(field('Webhook', '<code style="font-size:11px;word-break:break-all">' + esc(location.origin + job.webhook_url) + '</code> <button class="btn btn-ghost btn-sm" onclick="copyToClipboard(\'' + esc(location.origin + job.webhook_url) + '\',\'Webhook URL copied\')">Copy</button> <button class="btn btn-ghost btn-sm" style="color:var(--danger)" onclick="deleteWebhook(\'' + job.id + '\')">Disable</button>'));
    } else {
        extras.push(field('Webhook', '<button class="btn btn-ghost btn-sm" onclick="enableWebhook(\'' + job.id + '\')">Enable Webhook</button>'));
    }
    if (job.approval_required) extras.push(field('Approval', '<span class="badge badge-pending_approval">required</span>'));
    if (job.sla_deadline) extras.push(field('SLA', job.sla_deadline + ' UTC' + (job.sla_warning_mins ? ' (warn ' + job.sla_warning_mins + 'm)' : '')));
    if (job.starts_at || job.expires_at) {
        let windowHtml = '';
        if (job.starts_at) windowHtml += 'from ' + fmtDateUTC(job.starts_at);
        if (job.starts_at && job.expires_at) windowHtml += ' ';
        if (job.expires_at) windowHtml += 'until ' + fmtDateUTC(job.expires_at);
        extras.push(field('Window', windowHtml));
    }
    if (notifHtml.indexOf('off') === -1) extras.push(field('Alerts', notifHtml));
    if (rulesHtml.indexOf('none') === -1) extras.push(field('Output', rulesHtml));

    document.getElementById('detail-card').innerHTML =
        '<div class="card"><div class="card-header"><h3>' + esc(job.name) + ' ' + badge(job.status) +
        ' <span style="font-size:12px;font-weight:400;color:var(--accent)">' + esc(job.group || 'Default') + '</span></h3>' +
        '<div>' +
        (canWrite() ? '<button class="btn btn-ghost btn-sm" onclick="openEditModal(\'' + job.id + '\')">Edit</button> ' : '') +
        (canWrite() ? '<button class="btn btn-ghost btn-sm" onclick="copyJob(\'' + job.id + '\')">Copy</button> ' : '') +
        '<button class="btn btn-ghost btn-sm" onclick="showJobVersions(\'' + job.id + '\')">History</button> ' +
        (canWrite() ? '<button class="btn btn-ghost btn-sm" onclick="saveAsTemplate(\'' + job.id + '\')">Template</button> ' : '') +
        (canWrite() && job.status === 'scheduled' ? '<button class="btn btn-ghost btn-sm" onclick="togglePause(\'' + job.id + '\',\'scheduled\')">Pause</button> ' : '') +
        (canWrite() && job.status === 'paused' ? '<button class="btn btn-ghost btn-sm" onclick="togglePause(\'' + job.id + '\',\'paused\')">Resume</button> ' : '') +
        (canWrite() ? '<button class="btn btn-primary btn-sm" id="trigger-' + job.id + '" onclick="triggerJob(\'' + job.id + '\')">Trigger</button> ' : '') +
        (canWrite() ? '<button class="btn btn-danger btn-sm" onclick="deleteJob(\'' + job.id + '\',\'' + esc(job.name) + '\')">Delete</button>' : '') +
        '</div></div>' +
        (job.description ? '<div style="padding:0 16px 8px;color:var(--text-secondary);font-size:13px">' + esc(job.description) + '</div>' : '') +
        '<div class="detail-grid">' +
        field('Task', fmtTaskDetail(job.task)) +
        field('Schedule', fmtScheduleDetail(job.schedule)) +
        field('Target', fmtTarget(job.target)) +
        field('Next Fire', job.next_fire_time ? fmtDate(job.next_fire_time) : '-') +
        field('Last Run', lastHtml) +
        field('Runs', statsHtml) +
        field('Deps', deps) +
        field('Updated', fmtDate(job.updated_at)) +
        extras.join('') +
        '</div></div>';
}

async function enableWebhook(jobId) {
    try {
        const res = await api('POST', '/api/jobs/' + jobId + '/webhook');
        toast('Webhook enabled');
        showJobDetail(jobId);
    } catch (e) { toast(e.message, 'error'); }
}

async function deleteWebhook(jobId) {
    if (!confirm('Disable webhook for this job?')) return;
    try {
        await api('DELETE', '/api/jobs/' + jobId + '/webhook');
        toast('Webhook disabled');
        showJobDetail(jobId);
    } catch (e) { toast(e.message, 'error'); }
}

function field(label, value) {
    return '<div class="detail-field"><label>' + label + '</label><div class="value">' + value + '</div></div>';
}

// --- Actions ---
async function triggerJob(id, skipDeps) {
    // Fetch fresh job data to check for parameters (allJobs may be stale)
    if (!skipDeps) {
        try {
            const freshJob = await api('GET', '/api/jobs/' + id);
            if (freshJob.parameters && freshJob.parameters.length > 0) {
                showTriggerParamsModal(id, freshJob.parameters);
                return;
            }
        } catch (e) { /* proceed without params check */ }
    }
    const btn = document.getElementById('trigger-' + id);
    if (btn) btn.classList.add('trigger-pending');
    try {
        const qs = skipDeps ? '?skip_deps=true' : '';
        await api('POST', '/api/jobs/' + id + '/trigger' + qs);
        toast('Job triggered', 'info');
        // If on job detail, refresh to show live output
        if (currentJobId === id) {
            setTimeout(function() { showJobDetail(id); }, 1000);
        }
        // Poll rapidly for execution result
        pollForResult(id);
    } catch (e) {
        toast(e.message, 'error');
        if (btn) btn.classList.remove('trigger-pending');
    }
}

async function pollForResult(jobId) {
    let attempts = 0;
    const maxAttempts = 60; // 30 seconds max
    const interval = 500;
    const poll = async () => {
        attempts++;
        try {
            const res = await api('GET', '/api/jobs/' + jobId + '/executions?per_page=1&page=1');
            if (res.data.length > 0) {
                const latest = res.data[0];
                if (latest.status === 'succeeded') {
                    toast('Job succeeded (exit code 0)', 'success');
                    cleanup();
                    return;
                } else if (latest.status === 'failed') {
                    toast('Job failed (exit code ' + (latest.exit_code ?? '?') + ')', 'error');
                    cleanup();
                    return;
                } else if (latest.status === 'timed_out') {
                    toast('Job timed out', 'error');
                    cleanup();
                    return;
                } else if (latest.status === 'cancelled') {
                    toast('Job was cancelled');
                    cleanup();
                    return;
                }
            }
        } catch (e) { /* ignore poll errors */ }
        if (attempts < maxAttempts) {
            setTimeout(poll, interval);
        } else {
            toast('Job is still running...check back later');
            cleanup();
        }
    };
    const cleanup = () => {
        const btn = document.getElementById('trigger-' + jobId);
        if (btn) btn.classList.remove('trigger-pending');
        if (currentJobId === jobId) {
            // On job detail: refresh detail and switch to history tab
            showJobDetail(jobId);
            setTimeout(function() { setDetailTab('history'); }, 300);
        } else {
            fetchJobs();
        }
    };
    setTimeout(poll, interval);
}

async function togglePause(id, current) {
    const newStatus = current === 'scheduled' ? 'paused' : 'scheduled';
    try {
        await api('PUT', '/api/jobs/' + id, { status: newStatus });
        toast('Job ' + newStatus);
        if (currentJobId === id) showJobDetail(id);
        else fetchJobs();
    } catch (e) {
        toast(e.message, 'error');
    }
}

async function cancelLatestExecution(jobId) {
    const job = allJobs.find(j => j.id === jobId);
    if (!job || !job.last_execution || job.last_execution.status !== 'running') return;
    try {
        await api('POST', '/api/executions/' + job.last_execution.id + '/cancel');
        toast('Cancel request sent');
        fetchJobs();
    } catch (e) {
        toast(e.message, 'error');
    }
}

async function deleteJob(id, name) {
    if (!confirm('Delete job "' + name + '"? This will also delete all its execution history.')) return;
    try {
        await api('DELETE', '/api/jobs/' + id);
        toast('Job deleted');
        if (currentJobId === id) showJobsList();
        else fetchJobs();
    } catch (e) {
        toast(e.message, 'error');
    }
}

// --- Bulk Selection ---

function toggleSelectJob(checkbox) {
    const id = checkbox.dataset.id;
    if (checkbox.checked) {
        selectedJobs.add(id);
    } else {
        selectedJobs.delete(id);
    }
    updateBulkBar();
    // Update select-all checkbox state
    const all = document.querySelectorAll('#jobs-table-wrap .job-checkbox[data-id]');
    const selectAll = document.querySelector('#jobs-table-wrap thead .job-checkbox');
    if (selectAll) {
        selectAll.checked = all.length > 0 && selectedJobs.size === all.length;
    }
}

function toggleSelectAll(checkbox) {
    const all = document.querySelectorAll('#jobs-table-wrap .job-checkbox[data-id]');
    all.forEach(cb => {
        cb.checked = checkbox.checked;
        if (checkbox.checked) {
            selectedJobs.add(cb.dataset.id);
        } else {
            selectedJobs.delete(cb.dataset.id);
        }
    });
    updateBulkBar();
}

function updateBulkBar() {
    const countText = document.getElementById('bulk-count-text');
    const runBtn = document.getElementById('bulk-run-btn');
    const delBtn = document.getElementById('bulk-delete-btn');
    const clearBtn = document.getElementById('bulk-clear-btn');
    const groupBtn = document.getElementById('bulk-group-btn');
    const n = selectedJobs.size;
    runBtn.textContent = n > 0 ? '\u25B6 Schedule Now (' + n + ')' : '\u25B6 Schedule Now';
    delBtn.textContent = n > 0 ? 'Delete (' + n + ')' : 'Delete';
    if (groupBtn) groupBtn.textContent = n > 0 ? 'Set Group (' + n + ')' : 'Set Group';
    countText.textContent = n > 0 ? n + ' selected' : '';
    const disabled = n === 0;
    runBtn.disabled = disabled;
    delBtn.disabled = disabled;
    clearBtn.disabled = disabled;
    if (groupBtn) groupBtn.disabled = disabled;
}

function clearSelection() {
    selectedJobs.clear();
    renderJobsTable();
}

async function bulkTrigger() {
    const ids = Array.from(selectedJobs);
    const count = ids.length;
    if (!confirm('Schedule ' + count + ' job' + (count > 1 ? 's' : '') + ' now?')) return;
    let succeeded = 0;
    for (const id of ids) {
        try {
            await api('POST', '/api/jobs/' + id + '/trigger');
            succeeded++;
        } catch (e) {
            console.error('bulk trigger failed for ' + id, e);
        }
    }
    toast(succeeded + ' of ' + count + ' jobs triggered', succeeded === count ? 'success' : 'error');
    selectedJobs.clear();
    fetchJobs();
}

async function bulkDelete() {
    const ids = Array.from(selectedJobs);
    const count = ids.length;
    if (!confirm('Delete ' + count + ' job' + (count > 1 ? 's' : '') + '? This cannot be undone.')) return;
    let succeeded = 0;
    let errors = [];
    for (const id of ids) {
        try {
            await api('DELETE', '/api/jobs/' + id);
            succeeded++;
        } catch (e) {
            errors.push(e.message);
        }
    }
    if (errors.length > 0) {
        toast(succeeded + ' deleted, ' + errors.length + ' failed: ' + errors[0], 'error');
    } else {
        toast(succeeded + ' job' + (succeeded > 1 ? 's' : '') + ' deleted');
    }
    selectedJobs.clear();
    fetchJobs();
}

// --- Job Templates ---
function openTemplateJob(template) {
    closeWizard();
    openCreateModal();
    setTimeout(() => {
        if (template === 'health-check') {
            document.querySelector('input[name="task-type"][value="http"]').checked = true;
            updateTaskFields();
            document.getElementById('f-http-method').value = 'get';
            document.getElementById('f-http-url').value = 'https://example.com/health';
            document.getElementById('f-http-expect').value = '200';
            document.getElementById('f-name').value = 'health-check';
            document.querySelector('input[name="sched-type"][value="cron"]').checked = true;
            updateSchedFields();
        } else if (template === 'cron-task') {
            document.querySelector('input[name="task-type"][value="shell"]').checked = true;
            updateTaskFields();
            document.getElementById('f-command').value = 'echo "hello world"';
            document.getElementById('f-name').value = 'my-cron-job';
            document.querySelector('input[name="sched-type"][value="cron"]').checked = true;
            updateSchedFields();
        } else if (template === 'event-watcher') {
            document.getElementById('f-name').value = 'failure-alert';
            document.querySelector('input[name="task-type"][value="shell"]').checked = true;
            updateTaskFields();
            document.getElementById('f-command').value = 'echo "Job failed — investigate"';
            document.querySelector('input[name="sched-type"][value="event"]').checked = true;
            updateSchedFields();
            setEventKindValue('execution.completed');
            document.getElementById('f-event-severity').value = 'error';
        }
    }, 100);
}

async function showJobVersions(jobId) {
    try {
        const versions = await api('GET', '/api/jobs/' + jobId + '/versions');
        if (versions.length === 0) {
            toast('No version history for this job', 'info');
            return;
        }
        let html = '<div class="card"><div class="card-header"><h3>Version History</h3>' +
            '<button class="btn btn-ghost btn-sm" onclick="document.getElementById(\'version-modal\').style.display=\'none\'">Close</button></div>';
        html += '<div style="max-height:500px;overflow-y:auto;padding:12px">';
        for (const v of versions) {
            const snap = v.snapshot || {};
            const changes = [];
            if (snap.task) changes.push('task: ' + (snap.task.type || 'unknown'));
            if (snap.schedule) changes.push('schedule: ' + (typeof snap.schedule === 'string' ? snap.schedule : snap.schedule.type || JSON.stringify(snap.schedule)));
            if (snap.status) changes.push('status: ' + snap.status);
            html += '<div style="border-bottom:1px solid var(--border);padding:8px 0">';
            html += '<div style="display:flex;justify-content:space-between;align-items:center">';
            html += '<strong>v' + v.version + '</strong>';
            html += '<span style="font-size:11px;color:var(--text-muted)">' + fmtDate(v.created_at) + (v.changed_by ? ' by ' + esc(v.changed_by) : '') + '</span>';
            html += '</div>';
            html += '<div style="font-size:12px;color:var(--text-secondary);margin-top:4px">' + changes.map(esc).join(' | ') + '</div>';
            html += '</div>';
        }
        html += '</div></div>';
        // Show in a simple overlay
        let modal = document.getElementById('version-modal');
        if (!modal) {
            modal = document.createElement('div');
            modal.id = 'version-modal';
            modal.className = 'modal-overlay';
            modal.onclick = function(e) { if (e.target === modal) modal.style.display = 'none'; };
            modal.innerHTML = '<div class="modal-card" style="max-width:600px"></div>';
            document.body.appendChild(modal);
        }
        modal.querySelector('.modal-card').innerHTML = html;
        modal.style.display = '';
    } catch (e) {
        toast(e.message, 'error');
    }
}

// --- Templates ---

async function saveAsTemplate(jobId) {
    try {
        const job = await api('GET', '/api/jobs/' + jobId);
        const name = prompt('Template name:', job.name + '-template');
        if (!name) return;
        const description = prompt('Description (optional):', job.description || '');
        // Build snapshot: task + config, strip identity/schedule
        const snapshot = {
            task: job.task,
            target: job.target,
            output_rules: job.output_rules,
            notifications: job.notifications,
            group: job.group,
            run_as: job.run_as,
            timeout_secs: job.timeout_secs,
            retry_max: job.retry_max,
            retry_delay_secs: job.retry_delay_secs,
            retry_backoff: job.retry_backoff,
            priority: job.priority,
            approval_required: job.approval_required,
            sla_deadline: job.sla_deadline,
            sla_warning_mins: job.sla_warning_mins,
        };
        await api('POST', '/api/templates', { name, description: description || null, snapshot });
        toast('Template "' + name + '" saved');
    } catch (e) {
        toast(e.message, 'error');
    }
}

async function showTemplatesPicker() {
    try {
        const templates = await api('GET', '/api/templates');
        if (templates.length === 0) {
            toast('No saved templates. Save a job as template from the job detail page.', 'info');
            return;
        }
        let modal = document.getElementById('templates-modal');
        if (!modal) {
            modal = document.createElement('div');
            modal.id = 'templates-modal';
            modal.className = 'modal-overlay';
            modal.onclick = function(e) { if (e.target === modal) modal.style.display = 'none'; };
            modal.innerHTML = '<div class="modal-card" style="max-width:700px"></div>';
            document.body.appendChild(modal);
        }
        let html = '<div class="card"><div class="card-header"><h3>Job Templates</h3>' +
            '<button class="btn btn-ghost btn-sm" onclick="document.getElementById(\'templates-modal\').style.display=\'none\'">Close</button></div>';
        html += '<div style="padding:12px;display:grid;grid-template-columns:repeat(auto-fill,minmax(200px,1fr));gap:12px">';
        for (const t of templates) {
            const taskType = t.snapshot && t.snapshot.task ? (t.snapshot.task.type || 'unknown') : 'unknown';
            html += '<div class="card" style="cursor:pointer;border:1px solid var(--border);transition:border-color 0.15s" onclick="createFromTemplate(\'' + esc(t.name) + '\')" onmouseover="this.style.borderColor=\'var(--accent)\'" onmouseout="this.style.borderColor=\'var(--border)\'">';
            html += '<div style="padding:14px">';
            html += '<div style="font-weight:600;font-size:14px;margin-bottom:4px">' + esc(t.name) + '</div>';
            if (t.description) html += '<div style="font-size:12px;color:var(--text-secondary);margin-bottom:6px">' + esc(t.description) + '</div>';
            html += '<div style="display:flex;justify-content:space-between;align-items:center">';
            html += '<span class="badge badge-' + taskType + '" style="font-size:10px">' + taskType + '</span>';
            html += '<button class="btn btn-ghost btn-sm" style="color:var(--danger);font-size:11px;padding:2px 6px" onclick="event.stopPropagation();deleteTemplate(\'' + esc(t.name) + '\')">&times;</button>';
            html += '</div></div></div>';
        }
        html += '</div></div>';
        modal.querySelector('.modal-card').innerHTML = html;
        modal.style.display = '';
    } catch (e) {
        toast(e.message, 'error');
    }
}

async function createFromTemplate(templateName) {
    try {
        const t = await api('GET', '/api/templates/' + encodeURIComponent(templateName));
        // Close templates modal
        const modal = document.getElementById('templates-modal');
        if (modal) modal.style.display = 'none';
        // Open create modal and populate from snapshot
        openCreateModal();
        setTimeout(() => {
            const snap = t.snapshot;
            if (snap.task) populateTaskForm(snap.task);
            if (snap.group) populateGroupSelect(snap.group);
            if (snap.run_as) document.getElementById('f-run-as').value = snap.run_as;
            if (snap.timeout_secs) document.getElementById('f-timeout').value = snap.timeout_secs;
            if (snap.retry_max) document.getElementById('f-retry-max').value = snap.retry_max;
            if (snap.retry_delay_secs) document.getElementById('f-retry-delay').value = snap.retry_delay_secs;
            if (snap.retry_backoff) document.getElementById('f-retry-backoff').value = snap.retry_backoff;
            if (snap.priority) document.getElementById('f-priority').value = snap.priority;
            if (snap.approval_required) document.getElementById('f-approval-required').checked = true;
            if (snap.sla_deadline) document.getElementById('f-sla-deadline').value = snap.sla_deadline;
            if (snap.sla_warning_mins) document.getElementById('f-sla-warning').value = snap.sla_warning_mins;
            if (snap.output_rules) populateOutputRules(snap.output_rules);
            if (snap.notifications) populateJobNotifications(snap.notifications);
            // Set target
            if (snap.target) {
                if (snap.target.type === 'tagged') {
                    document.getElementById('f-target-type').value = 'tagged';
                    document.getElementById('f-target-tag').value = snap.target.tag || '';
                } else if (snap.target.type) {
                    document.getElementById('f-target-type').value = snap.target.type;
                }
            }
        }, 100);
    } catch (e) {
        toast(e.message, 'error');
    }
}

async function deleteTemplate(name) {
    if (!confirm('Delete template "' + name + '"?')) return;
    try {
        await api('DELETE', '/api/templates/' + encodeURIComponent(name));
        toast('Template deleted');
        showTemplatesPicker(); // Refresh
    } catch (e) {
        toast(e.message, 'error');
    }
}

