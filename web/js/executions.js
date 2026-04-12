// Kronforce - Execution list, detail, diff view, polling
// --- Executions ---
async function fetchExecutions(jobId, resetPage) {
    if (resetPage) execsPage = 1;
    try {
        const res = await api('GET', '/api/jobs/' + jobId + '/executions?page=' + execsPage + '&per_page=' + PER_PAGE);
        execsTotalPages = res.total_pages;
        execsTotal = res.total;
        renderExecTable(res.data);
        renderPagination('exec-pagination', execsPage, execsTotalPages, execsTotal, goToExecsPage);
    } catch (e) {
        console.error('fetchExecutions:', e);
    }
}

function goToExecsPage(p) {
    execsPage = p;
    if (currentJobId) fetchExecutions(currentJobId);
}

let execSortColumn = 'started_at';
let execSortDirection = 'desc';

function sortExecs(col) {
    if (execSortColumn === col) {
        execSortDirection = execSortDirection === 'asc' ? 'desc' : 'asc';
    } else {
        execSortColumn = col;
        execSortDirection = col === 'started_at' ? 'desc' : 'asc';
    }
    fetchAllExecutions();
}

function getExecSortValue(e, col) {
    switch (col) {
        case 'job': return resolveJobName(e.job_id);
        case 'status': return e.status;
        case 'exit_code': return e.exit_code !== null ? e.exit_code : -999;
        case 'started_at': return e.started_at || '';
        case 'duration': {
            if (!e.started_at || !e.finished_at) return 0;
            return new Date(e.finished_at) - new Date(e.started_at);
        }
        default: return '';
    }
}

function renderExecTable(execs, { wrapId, showJobColumn, emptyMessage } = {}) {
    const wrap = document.getElementById(wrapId || 'exec-table-wrap');
    if (execs.length === 0) {
        wrap.innerHTML = '<div class="empty-state"><p>' + (emptyMessage || 'No executions yet') + '</p></div>';
        return;
    }

    // Sort if on the all-executions page
    if (showJobColumn) {
        execs = [...execs].sort((a, b) => {
            let va = getExecSortValue(a, execSortColumn);
            let vb = getExecSortValue(b, execSortColumn);
            if (typeof va === 'number' && typeof vb === 'number') {
                return execSortDirection === 'asc' ? va - vb : vb - va;
            }
            va = String(va); vb = String(vb);
            const cmp = va.localeCompare(vb);
            return execSortDirection === 'asc' ? cmp : -cmp;
        });
    }

    function execSortTh(col, label) {
        const cls = execSortColumn === col ? (execSortDirection === 'asc' ? 'sortable sort-asc' : 'sortable sort-desc') : 'sortable';
        return '<th class="' + cls + '" onclick="sortExecs(\'' + col + '\')">' + label + '</th>';
    }

    let html = '<table><thead><tr><th>ID</th>';
    if (showJobColumn) html += execSortTh('job', 'Job');
    else if (showJobColumn === false) { /* skip */ }
    html += execSortTh('status', 'Status');
    html += '<th>Exit Code</th>';
    html += execSortTh('started_at', 'Started');
    html += execSortTh('duration', 'Duration');
    html += '<th>Agent</th><th>Trigger</th></tr></thead><tbody>';

    // Track latest execution per job to mark it
    const latestByJob = {};
    for (const e of execs) {
        if (!latestByJob[e.job_id]) latestByJob[e.job_id] = e.id;
    }
    for (const e of execs) {
        const isLatest = latestByJob[e.job_id] === e.id;
        html += '<tr style="cursor:pointer' + (isLatest ? ';border-left:3px solid var(--accent)' : '') + '" onclick="showExecDetail(\'' + e.id + '\')">';
        html += '<td><span class="schedule-text" title="' + e.id + '">' + e.id.slice(0, 8) + (isLatest ? ' <span style="font-size:9px;color:var(--accent)">latest</span>' : '') + '</span></td>';
        if (showJobColumn) {
            const jobName = resolveJobName(e.job_id);
            html += '<td><span class="job-name" onclick="event.stopPropagation();showJobDetail(\'' + e.job_id + '\')">' + esc(jobName) + '</span></td>';
        }
        html += '<td>' + execBadge(e.status, e.agent_id) + '</td>';
        html += '<td>' + (e.exit_code !== null ? e.exit_code : '-') + '</td>';
        html += '<td><span class="time-text">' + (e.started_at ? fmtDate(e.started_at) : '-') + '</span></td>';
        html += '<td><span class="time-text">' + fmtDuration(e.started_at, e.finished_at) + '</span></td>';
        html += '<td>' + (e.agent_id ? fmtAgentLink(e.agent_id) : '<span class="time-text">controller</span>') + '</td>';
        html += '<td><span class="time-text">' + fmtTrigger(e.triggered_by) + '</span></td>';
        html += '</tr>';
    }
    html += '</tbody></table>';
    wrap.innerHTML = html;
}

async function showExecDetail(id) {
    currentExecId = id;
    if (typeof updateHash === 'function') updateHash();
    try {
        const e = await api('GET', '/api/executions/' + id);
        const content = document.getElementById('exec-detail-content');
        const jobName = resolveJobName(e.job_id);
        content.innerHTML =
            '<div class="exec-info">' +
            infoField('Job', '<span class="job-name" style="cursor:pointer" onclick="closeExecModal();showJobDetail(\'' + e.job_id + '\')">' + esc(jobName) + '</span>', 'exec-info-item') +
            infoField('Status', execBadge(e.status, e.agent_id), 'exec-info-item') +
            infoField('Exit Code', e.exit_code !== null ? e.exit_code : '-', 'exec-info-item') +
            infoField('Started', e.started_at ? fmtDateUTC(e.started_at) : '-', 'exec-info-item') +
            infoField('Finished', e.finished_at ? fmtDateUTC(e.finished_at) : '-', 'exec-info-item') +
            infoField('Duration', fmtDuration(e.started_at, e.finished_at), 'exec-info-item') +
            infoField('Agent', e.agent_id ? fmtAgentLink(e.agent_id) : 'controller', 'exec-info-item') +
            infoField('Trigger', fmtTrigger(e.triggered_by), 'exec-info-item') +
            '</div>' +
            (e.params ? '<div class="output-section"><h4>Parameters</h4><pre class="output-pre">' + syntaxHighlightJson(esc(JSON.stringify(e.params, null, 2))) + '</pre></div>' : '') +
            renderExtractedValues(e.extracted) +
            (e.task_snapshot ? '<div class="output-section"><h4>Task Executed</h4><pre class="output-pre">' + syntaxHighlightJson(esc(JSON.stringify(sanitizeTaskSnapshot(e.task_snapshot), null, 2))) + '</pre></div>' : '') +
            renderOutputSection('Output', e.stdout, e.stdout_truncated) +
            renderOutputSection('Error', e.stderr, e.stderr_truncated) +
            '<div id="diff-section" style="margin-top:12px"><button class="btn btn-ghost btn-sm" onclick="showOutputDiff(\'' + e.job_id + '\',\'' + e.id + '\')">Compare with previous run</button></div>';
        window._currentExecStdout = e.stdout || '';

        // Live output streaming for running executions
        if (e.status === 'running' && !e.agent_id) {
            content.innerHTML += '<div class="output-section" id="live-output-section"><h4>Live Output</h4><pre class="output-pre" id="live-output" style="max-height:400px;overflow-y:auto;font-size:11px"></pre></div>';
        }

        document.getElementById('exec-cancel-btn').style.display = e.status === 'running' ? '' : 'none';
        document.getElementById('exec-approve-btn').style.display = e.status === 'pending_approval' ? '' : 'none';
        openModal('exec-modal');

        if (e.status === 'running' && !e.agent_id) {
            startLiveStream(id);
        }
    } catch (e) {
        toast(e.message, 'error');
    }
}

function sanitizeTaskSnapshot(snap) {
    if (!snap || snap.type !== 'file_push') return snap;
    const copy = Object.assign({}, snap);
    if (copy.content_base64) {
        const size = Math.floor(copy.content_base64.length * 3 / 4);
        copy.content_base64 = '[' + (size / 1024).toFixed(1) + ' KB]';
    }
    return copy;
}

function renderExtractedValues(extracted) {
    if (!extracted || typeof extracted !== 'object' || Object.keys(extracted).length === 0) return '';
    let html = '<div class="output-section"><h4>Extracted Values</h4><div class="exec-info">';
    for (const [key, val] of Object.entries(extracted)) {
        html += infoField(key, esc(String(val)), 'exec-info-item');
    }
    html += '</div></div>';
    return html;
}

async function showOutputDiff(jobId, currentExecId) {
    const section = document.getElementById('diff-section');
    // Fetch recent executions for this job
    try {
        const res = await api('GET', '/api/jobs/' + jobId + '/executions?per_page=10');
        const others = res.data.filter(e => e.id !== currentExecId && e.status !== 'pending' && e.status !== 'running');
        if (others.length === 0) {
            section.innerHTML = '<div class="form-hint">No previous completed runs to compare</div>';
            return;
        }
        let html = '<div style="margin-bottom:8px"><select id="diff-select" style="width:300px"><option value="">Select a run to compare...</option>';
        for (const e of others) {
            html += '<option value="' + e.id + '">' + e.id.slice(0, 8) + ' \u2022 ' + e.status + ' \u2022 ' + (e.started_at ? fmtDate(e.started_at) : '?') + '</option>';
        }
        html += '</select> <button class="btn btn-primary btn-sm" onclick="runOutputDiff(\'' + currentExecId + '\')">Diff</button></div>';
        html += '<div id="diff-output"></div>';
        section.innerHTML = html;
    } catch (e) {
        section.innerHTML = '<div class="form-hint" style="color:var(--danger)">Failed to load executions</div>';
    }
}

async function runOutputDiff(currentExecId) {
    const otherId = document.getElementById('diff-select').value;
    if (!otherId) { toast('Select a run to compare', 'error'); return; }
    const output = document.getElementById('diff-output');
    try {
        const other = await api('GET', '/api/executions/' + otherId);
        const currentStdout = window._currentExecStdout || '';
        const otherStdout = other.stdout || '';
        // Truncate for diff
        const maxLen = 50000;
        const a = otherStdout.length > maxLen ? otherStdout.slice(0, maxLen) + '\n[...truncated for diff]' : otherStdout;
        const b = currentStdout.length > maxLen ? currentStdout.slice(0, maxLen) + '\n[...truncated for diff]' : currentStdout;
        const diff = computeDiff(a.split('\n'), b.split('\n'));
        output.innerHTML = renderDiffView(diff);
    } catch (e) {
        output.innerHTML = '<div class="form-hint" style="color:var(--danger)">Failed to load execution</div>';
    }
}

function computeDiff(linesA, linesB) {
    // Simple LCS-based line diff
    const m = linesA.length, n = linesB.length;
    // For large inputs, fall back to simple sequential comparison
    if (m * n > 1000000) {
        const result = [];
        const max = Math.max(m, n);
        for (let i = 0; i < max; i++) {
            if (i < m && i < n && linesA[i] === linesB[i]) {
                result.push({ type: 'same', line: linesA[i] });
            } else {
                if (i < m) result.push({ type: 'remove', line: linesA[i] });
                if (i < n) result.push({ type: 'add', line: linesB[i] });
            }
        }
        return result;
    }
    // LCS table
    const dp = Array(m + 1).fill(null).map(() => Array(n + 1).fill(0));
    for (let i = 1; i <= m; i++) {
        for (let j = 1; j <= n; j++) {
            dp[i][j] = linesA[i-1] === linesB[j-1] ? dp[i-1][j-1] + 1 : Math.max(dp[i-1][j], dp[i][j-1]);
        }
    }
    // Backtrack
    const result = [];
    let i = m, j = n;
    while (i > 0 || j > 0) {
        if (i > 0 && j > 0 && linesA[i-1] === linesB[j-1]) {
            result.unshift({ type: 'same', line: linesA[i-1] });
            i--; j--;
        } else if (j > 0 && (i === 0 || dp[i][j-1] >= dp[i-1][j])) {
            result.unshift({ type: 'add', line: linesB[j-1] });
            j--;
        } else {
            result.unshift({ type: 'remove', line: linesA[i-1] });
            i--;
        }
    }
    return result;
}

function renderDiffView(diff) {
    let html = '<div class="diff-view">';
    let lineNum = 0;
    for (const d of diff) {
        lineNum++;
        const cls = d.type === 'add' ? 'diff-add' : d.type === 'remove' ? 'diff-remove' : 'diff-same';
        const prefix = d.type === 'add' ? '+' : d.type === 'remove' ? '-' : ' ';
        html += '<div class="' + cls + '"><span class="diff-ln">' + lineNum + '</span><span class="diff-prefix">' + prefix + '</span>' + esc(d.line) + '</div>';
    }
    if (diff.every(d => d.type === 'same')) {
        html = '<div class="form-hint" style="color:var(--success)">No differences</div>' + html;
    }
    html += '</div>';
    return html;
}

let outputIdCounter = 0;

// Store raw output content by id
const outputRawData = {};

function detectOutputType(raw) {
    const trimmed = raw.trim();
    if (!trimmed || trimmed === '(empty)') return 'text';
    // Check JSON
    if ((trimmed.startsWith('{') && trimmed.endsWith('}')) || (trimmed.startsWith('[') && trimmed.endsWith(']'))) {
        try { JSON.parse(trimmed); return 'json'; } catch(e) {}
    }
    // Check HTML - look for common tags
    if (/<(!DOCTYPE|html|head|body|div|span|p|h[1-6]|table|ul|ol|a |img |form|input|script|style|link|meta)/i.test(trimmed)) {
        return 'html';
    }
    return 'text';
}

function downloadOutput(id, label) {
    const raw = outputRawData[id];
    if (!raw) return;
    const blob = new Blob([raw], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = (currentExecId ? currentExecId.slice(0, 8) : 'output') + '-' + label + '.log';
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
}

function syntaxHighlightJson(json) {
    // Expects already-escaped HTML of pretty-printed JSON
    return json
        .replace(/("(?:\\.|[^"\\])*")\s*:/g, '<span style="color:#3e8bff">$1</span>:')  // keys
        .replace(/:\s*("(?:\\.|[^"\\])*")/g, ': <span style="color:#2ecc71">$1</span>')  // string values
        .replace(/:\s*(\d+\.?\d*)/g, ': <span style="color:#e6a817">$1</span>')           // numbers
        .replace(/:\s*(true|false)/g, ': <span style="color:#e67e22">$1</span>')           // booleans
        .replace(/:\s*(null)/g, ': <span style="color:#7c8298">$1</span>');                // null
}

function renderOutputSection(label, content, truncated) {
    const id = 'output-' + (outputIdCounter++);
    const truncBadge = truncated ? ' <span class="badge badge-paused">truncated</span>' : '';
    const raw = content || '(empty)';
    outputRawData[id] = raw;

    const detected = detectOutputType(raw);

    let html = '<div class="output-section"><h4>' + label + truncBadge + '</h4>';
    html += '<div class="output-tabs">';
    html += '<button class="output-tab' + (detected === 'text' ? ' active' : '') + '" onclick="switchOutputView(\'' + id + '\',\'text\',this)">Text</button>';
    html += '<button class="output-tab' + (detected === 'json' ? ' active' : '') + '" onclick="switchOutputView(\'' + id + '\',\'json\',this)">JSON</button>';
    html += '<button class="output-tab' + (detected === 'html' ? ' active' : '') + '" onclick="switchOutputView(\'' + id + '\',\'html\',this)">HTML</button>';
    if (content && content !== '(empty)') {
        html += '<button class="output-tab" onclick="downloadOutput(\'' + id + '\',\'' + esc(label).toLowerCase() + '\')" title="Download as file">&#11015; Download</button>';
    }
    html += '</div>';

    // Render the detected format by default
    if (detected === 'json') {
        let formatted;
        try { formatted = JSON.stringify(JSON.parse(raw.trim()), null, 2); } catch(e) { formatted = raw; }
        html += '<div id="' + id + '-wrap"><pre class="output-pre">' + syntaxHighlightJson(esc(formatted)) + '</pre></div>';
    } else if (detected === 'html') {
        html += '<div id="' + id + '-wrap"><iframe class="output-iframe" sandbox="allow-same-origin" srcdoc="' + esc(raw).replace(/"/g, '&quot;') + '"></iframe></div>';
    } else {
        html += '<div id="' + id + '-wrap"><pre class="output-pre">' + esc(raw) + '</pre></div>';
    }

    html += '</div>';
    return html;
}

function switchOutputView(id, mode, btn) {
    btn.parentElement.querySelectorAll('.output-tab').forEach(t => t.classList.remove('active'));
    btn.classList.add('active');

    const wrap = document.getElementById(id + '-wrap');
    const raw = outputRawData[id] || '';

    if (mode === 'text') {
        wrap.innerHTML = '<pre class="output-pre">' + esc(raw) + '</pre>';
    } else if (mode === 'json') {
        let formatted;
        try {
            const parsed = JSON.parse(raw.trim());
            formatted = JSON.stringify(parsed, null, 2);
        } catch(e) {
            formatted = '(not valid JSON)\n\n' + raw;
        }
        wrap.innerHTML = '<pre class="output-pre">' + syntaxHighlightJson(esc(formatted)) + '</pre>';
    } else if (mode === 'html') {
        wrap.innerHTML = '<iframe class="output-iframe" sandbox="allow-same-origin" srcdoc="' + esc(raw).replace(/"/g, '&quot;') + '"></iframe>';
    }
}

function infoField(label, value, className) {
    return '<div class="' + className + '"><label>' + label + '</label><div class="value">' + value + '</div></div>';
}

let liveEventSource = null;

function startLiveStream(execId) {
    stopLiveStream();
    const pre = document.getElementById('live-output');
    if (!pre) return;

    liveEventSource = new EventSource('/api/executions/' + execId + '/stream');
    liveEventSource.onmessage = function(event) {
        pre.textContent += event.data + '\n';
        pre.scrollTop = pre.scrollHeight;
    };
    liveEventSource.addEventListener('done', function() {
        stopLiveStream();
        // Refresh to show final static output
        showExecDetail(execId);
    });
    liveEventSource.onerror = function() {
        stopLiveStream();
    };
}

function stopLiveStream() {
    if (liveEventSource) {
        liveEventSource.close();
        liveEventSource = null;
    }
}

function closeExecModal() {
    stopLiveStream();
    closeModal('exec-modal');
    currentExecId = null;
    if (typeof updateHash === 'function') updateHash();
}

async function showWaitingDetail(jobId) {
    try {
        const job = await api('GET', '/api/jobs/' + jobId);
        const content = document.getElementById('waiting-detail-content');

        if (!job.deps_status || job.deps_status.length === 0) {
            content.innerHTML = '<p style="color:var(--text-secondary)">No dependencies configured.</p>';
        } else {
            let html = '<p style="margin-bottom:12px;color:var(--text-secondary)">This job will run once all dependencies are satisfied:</p>';
            html += '<div style="display:flex;flex-direction:column;gap:8px">';
            for (const d of job.deps_status) {
                const name = d.job_name || d.job_id.slice(0, 8);
                const icon = d.satisfied ? '\u2714' : '\u2718';
                const color = d.satisfied ? 'var(--success)' : 'var(--danger)';
                const statusText = d.satisfied ? 'Satisfied' : 'Not met';
                const window = d.within_secs ? ' within ' + fmtSeconds(d.within_secs) : '';

                html += '<div style="display:flex;align-items:center;gap:10px;padding:10px;background:var(--bg-tertiary);border-radius:var(--radius);border:1px solid var(--border)">';
                html += '<span style="font-size:18px;color:' + color + '">' + icon + '</span>';
                html += '<div style="flex:1">';
                html += '<div style="font-weight:600;font-size:13px"><span class="job-name" onclick="closeWaitingModal();showJobDetail(\'' + d.job_id + '\')">' + esc(name) + '</span></div>';
                html += '<div style="font-size:11px;color:var(--text-muted)">Must have succeeded' + window + '</div>';
                html += '</div>';
                html += '<span class="badge ' + (d.satisfied ? 'badge-succeeded' : 'badge-failed') + '">' + statusText + '</span>';
                html += '</div>';
            }
            html += '</div>';
            content.innerHTML = html;
        }

        // Show "Run Anyway" button if deps are not all satisfied
        const runAnywayBtn = document.getElementById('waiting-run-anyway-btn');
        if (runAnywayBtn) {
            runAnywayBtn.style.display = job.deps_satisfied ? 'none' : '';
            runAnywayBtn.onclick = function() {
                closeWaitingModal();
                triggerJob(jobId, true);
            };
        }

        openModal('waiting-modal');
    } catch (e) {
        toast(e.message, 'error');
    }
}

function formField({ type, id, label, placeholder, hint, style, options }) {
    let html = '<div class="form-group">';
    html += '<label>' + label + '</label>';
    if (type === 'textarea') {
        html += '<textarea id="' + id + '" placeholder="' + (placeholder || '') + '"' + (style ? ' style="' + style + '"' : '') + '></textarea>';
    } else if (type === 'select') {
        html += '<select id="' + id + '"' + (style ? ' style="' + style + '"' : '') + '>';
        for (const o of (options || [])) {
            html += '<option value="' + (o.value !== undefined ? o.value : o.label) + '">' + o.label + '</option>';
        }
        html += '</select>';
    } else {
        html += '<input id="' + id + '" type="' + (type || 'text') + '" placeholder="' + (placeholder || '') + '"' + (style ? ' style="' + style + '"' : '') + '>';
    }
    if (hint) html += '<div class="form-hint">' + hint + '</div>';
    html += '</div>';
    return html;
}

// --- All Executions ---

let allExecsPage = 1;
const execSearch = createSearchFilter({ inputId: 'exec-search-input', clearBtnId: 'exec-search-clear', filterContainerId: 'exec-status-filters', onUpdate: () => { allExecsPage = 1; fetchAllExecutions(); } });

async function fetchAllExecutions() {
    try {
        // Ensure job names are available for resolving job_id → name
        // Fetch all jobs (not just first page) so names resolve correctly
        try {
            const jobsRes = await api('GET', '/api/jobs?per_page=1000');
            allJobs = jobsRes.data;
        } catch (_) {}
        let qs = '?page=' + allExecsPage + '&per_page=' + PER_PAGE;
        if (execSearch.statusFilter) qs += '&status=' + execSearch.statusFilter;
        if (execSearch.searchTerm) qs += '&search=' + encodeURIComponent(execSearch.searchTerm);
        if (timeRanges.execs) qs += '&since=' + encodeURIComponent(getSinceISO(timeRanges.execs));
        const res = await api('GET', '/api/executions' + qs);
        if (res.data.length === 0 && !execSearch.statusFilter && !execSearch.searchTerm) {
            document.getElementById('all-execs-table-wrap').innerHTML = renderRichEmptyState({
                icon: '&#9654;',
                title: 'No executions yet',
                description: 'Executions appear when jobs run. Create a job and trigger it to see results here.',
                actions: [
                    { label: 'Go to Jobs', onclick: "showPage('jobs')", primary: true },
                ],
            });
        } else {
            renderExecTable(res.data, { wrapId: 'all-execs-table-wrap', showJobColumn: true, emptyMessage: 'No executions match your filters' });
        }
        renderPagination('all-execs-pagination', allExecsPage, res.total_pages, res.total, goToAllExecsPage);
    } catch (e) {
        console.error('fetchAllExecutions:', e);
    }
}

function goToAllExecsPage(p) {
    allExecsPage = p;
    fetchAllExecutions();
}

