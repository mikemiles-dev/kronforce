// Kronforce - Job CRUD, list, detail, actions, bulk selection, templates
// --- Job Groups ---
let cachedGroups = [];
let groupFilter = '';

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
        const sel = document.getElementById('group-filter');
        if (!sel) return;
        sel.innerHTML = '<option value="">All Groups</option>';
        for (const g of cachedGroups) {
            sel.innerHTML += '<option value="' + esc(g) + '"' + (groupFilter === g ? ' selected' : '') + '>' + esc(g) + '</option>';
        }
    } catch (e) {
        console.error('fetchGroups:', e);
    }
}

function setGroupFilter(value) {
    groupFilter = value;
    fetchJobs(true);
}

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
        const isClientFilter = filter === 'blocked';
        const search = jobSearch.searchTerm;
        // "scheduled" filter should include waiting (scheduled) jobs, so send scheduled to API
        // "blocked" is client-side only, fetch all
        const apiFilter = filter === 'scheduled' ? 'scheduled' : (isClientFilter ? '' : filter);
        let qs = '?page=' + jobsPage + '&per_page=' + (isClientFilter ? 100 : PER_PAGE);
        if (apiFilter) qs += '&status=' + apiFilter;
        if (search) qs += '&search=' + encodeURIComponent(search);
        if (groupFilter) qs += '&group=' + encodeURIComponent(groupFilter);
        const res = await api('GET', '/api/jobs' + qs);
        allJobs = res.data;
        if (isClientFilter) {
            // Waiting = has deps that aren't satisfied
            allJobs = allJobs.filter(j => j.depends_on.length > 0 && !j.deps_satisfied);
        } else if (filter === 'unscheduled') {
            allJobs = allJobs.filter(j => !(j.depends_on.length > 0 && !j.deps_satisfied));
        }
        // Time range filter (client-side for jobs - filter by last execution time)
        if (timeRanges.jobs) {
            const since = new Date(Date.now() - parseInt(timeRanges.jobs) * 60000);
            allJobs = allJobs.filter(j => {
                if (!j.last_execution || !j.last_execution.finished_at) return false;
                return new Date(j.last_execution.finished_at) >= since;
            });
        }
        const isTimeFiltered = !!timeRanges.jobs;
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
        const isBlocked = j.depends_on.length > 0 && !j.deps_satisfied;
        if (isBlocked) {
            html += '<td><span class="badge badge-paused" style="cursor:pointer" onclick="showWaitingDetail(\'' + j.id + '\')" title="Click to see what this job is waiting for">waiting</span></td>';
        } else {
            html += '<td>' + badge(j.status) + '</td>';
        }
        html += '<td>' + fmtTarget(j.target) + '</td>';
        html += '<td>' + fmtLastRun(j.last_execution) + '</td>';
        html += '<td>' + fmtCounts(j.execution_counts) + '</td>';
        html += '<td><span class="schedule-text">' + fmtSchedule(j.schedule) + '</span></td>';
        html += '<td><span class="time-text">' + (j.next_fire_time ? fmtDate(j.next_fire_time) : '-') + '</span></td>';
        html += '<td><div class="actions">';
        html += '<button class="btn-icon trigger" title="Trigger" id="trigger-' + j.id + '" onclick="triggerJob(\'' + j.id + '\')">&#9654;</button>';
        if (j.status === 'scheduled') {
            html += '<button class="btn-icon" title="Pause" onclick="togglePause(\'' + j.id + '\',\'scheduled\')">&#10074;&#10074;</button>';
        } else if (j.status === 'paused') {
            html += '<button class="btn-icon trigger" title="Resume" onclick="togglePause(\'' + j.id + '\',\'paused\')">&#9654;</button>';
        }
        html += '<button class="btn-icon danger" title="Delete" onclick="deleteJob(\'' + j.id + '\',\'' + esc(j.name) + '\')">&#128465;</button>';
        html += '</div></td>';
        html += '</tr>';
    }
    html += '</tbody></table>';
    wrap.innerHTML = html;
    updateBulkBar();
}

// --- Job Detail ---
let detailReturnTo = 'jobs';

async function showJobDetail(id) {
    detailReturnTo = currentPage;
    currentJobId = id;
    execsPage = 1;
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
        await fetchExecutions(id, true);
    } catch (e) {
        toast(e.message, 'error');
    }
}

function showJobsList() {
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

    document.getElementById('detail-card').innerHTML =
        '<div class="card"><div class="card-header"><h3>' + esc(job.name) + ' ' + badge(job.status) + '</h3>' +
        '<div><button class="btn btn-ghost btn-sm" onclick="openEditModal(\'' + job.id + '\')">Edit</button> ' +
        '<button class="btn btn-ghost btn-sm" onclick="copyJob(\'' + job.id + '\')">Copy</button> ' +
        '<button class="btn btn-ghost btn-sm" onclick="showJobVersions(\'' + job.id + '\')">History</button> ' +
        '<button class="btn btn-primary btn-sm" onclick="triggerJob(\'' + job.id + '\')">Trigger</button></div></div>' +
        '<div class="detail-grid">' +
        field('Task', fmtTaskDetail(job.task)) +
        field('Schedule', fmtScheduleDetail(job.schedule)) +
        field('Description', job.description || '-') +
        field('Run As', job.run_as ? '<code>' + esc(job.run_as) + '</code>' : 'Default') +
        field('Timeout', job.timeout_secs ? job.timeout_secs + 's' : 'None') +
        field('Target', fmtTarget(job.target)) +
        field('Dependencies', deps) +
        (job.priority ? field('Priority', String(job.priority)) : '') +
        (job.approval_required ? field('Approval', '<span class="badge badge-pending_approval">required</span>') : '') +
        field('Next Fire', job.next_fire_time ? fmtDate(job.next_fire_time) : '-') +
        field('Created', fmtDate(job.created_at)) +
        field('Updated', fmtDate(job.updated_at)) +
        '</div></div>';
}

function field(label, value) {
    return '<div class="detail-field"><label>' + label + '</label><div class="value">' + value + '</div></div>';
}

// --- Actions ---
async function triggerJob(id) {
    const btn = document.getElementById('trigger-' + id);
    if (btn) btn.classList.add('trigger-pending');
    try {
        await api('POST', '/api/jobs/' + id + '/trigger');
        toast('Job triggered, waiting for result...', 'info');
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
        fetchJobs();
        if (currentJobId) fetchExecutions(currentJobId);
    };
    setTimeout(poll, interval);
}

async function togglePause(id, current) {
    const newStatus = current === 'scheduled' ? 'paused' : 'scheduled';
    try {
        await api('PUT', '/api/jobs/' + id, { status: newStatus });
        toast('Job ' + newStatus);
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

