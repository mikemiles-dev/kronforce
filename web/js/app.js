// Kronforce - Core infrastructure (global state, utilities, auth, theme, routing, init)
let allJobs = [];
let selectedJobs = new Set();
let sortColumn = 'name';
let sortDirection = 'asc';
let currentJobId = null;
let editingJobId = null;
let currentExecId = null;
let pollTimer = null;
let jobsPage = 1;

// View registry for page management
const ALL_VIEWS = ['dashboard','jobs','detail','agents','executions','scripts','events','variables','connections','settings','docs','guide'];
const VIEW_ACTION_BARS = { jobs: 'jobs-action-bar', agents: 'agents-action-bar', executions: 'executions-action-bar', events: 'events-action-bar' };

// Time range state — persisted in localStorage
const timeRanges = JSON.parse(localStorage.getItem('kf-timeRanges') || '{"jobs":"","execs":"","events":""}');
const timeRangeCustom = JSON.parse(localStorage.getItem('kf-timeRangeCustom') || '{"jobs":{"from":"","to":""},"execs":{"from":"","to":""},"events":{"from":"","to":""}}');
function persistTimeRanges() { localStorage.setItem('kf-timeRanges', JSON.stringify(timeRanges)); localStorage.setItem('kf-timeRangeCustom', JSON.stringify(timeRangeCustom)); }
let activePopup = null;

const quickRanges = [
    { label: 'Last 15m', value: '15' },
    { label: 'Last 1h', value: '60' },
    { label: 'Last 6h', value: '360' },
    { label: 'Last 12h', value: '720' },
    { label: 'Last 24h', value: '1440' },
    { label: 'Last 7d', value: '10080' },
    { label: 'Last 30d', value: '43200' },
    { label: 'Today', value: 'today' },
    { label: 'All time', value: '' },
];

function toggleTimeRangePopup(scope, e) {
    if (e) e.stopPropagation();
    const popup = document.getElementById('tr-popup-' + scope);
    if (popup.style.display !== 'none') {
        popup.style.display = 'none';
        activePopup = null;
        return;
    }
    document.querySelectorAll('.time-range-popup').forEach(p => p.style.display = 'none');
    renderTimeRangePopup(scope);
    popup.style.display = '';
    activePopup = scope;
    // Prevent closing immediately
    if (!popup._hasListener) {
        popup.addEventListener('mousedown', function(ev) { ev.stopPropagation(); });
        popup._hasListener = true;
    }
}

function renderTimeRangePopup(scope) {
    const popup = document.getElementById('tr-popup-' + scope);
    const currentTab = popup.dataset.tab || 'quick';

    let html = '<div class="tr-tabs">';
    html += '<button class="tr-tab' + (currentTab === 'quick' ? ' active' : '') + '" onclick="switchTrTab(\'' + scope + '\',\'quick\')">Quick</button>';
    html += '<button class="tr-tab' + (currentTab === 'custom' ? ' active' : '') + '" onclick="switchTrTab(\'' + scope + '\',\'custom\')">Custom</button>';
    html += '</div>';

    if (currentTab === 'quick') {
        html += '<div class="tr-quick-buttons">';
        for (const r of quickRanges) {
            const active = timeRanges[scope] === r.value ? ' active' : '';
            html += '<button class="tr-quick-btn' + active + '" onclick="applyQuickRange(\'' + scope + '\',\'' + r.value + '\')">' + r.label + '</button>';
        }
        html += '</div>';
    } else {
        const cust = timeRangeCustom[scope];
        html += '<div class="tr-custom">';
        html += '<div><label>From</label><input type="datetime-local" id="tr-from-' + scope + '" value="' + (cust.from || '') + '"></div>';
        html += '<div><label>To</label><input type="datetime-local" id="tr-to-' + scope + '" value="' + (cust.to || '') + '"></div>';
        html += '<button class="btn btn-primary btn-sm tr-apply" onclick="applyCustomRange(\'' + scope + '\')">Apply</button>';
        html += '</div>';
    }

    popup.innerHTML = html;
}

function switchTrTab(scope, tab) {
    const popup = document.getElementById('tr-popup-' + scope);
    popup.dataset.tab = tab;
    renderTimeRangePopup(scope);
}

function applyQuickRange(scope, value) {
    if (value === 'today') {
        const now = new Date();
        const start = new Date(now.getFullYear(), now.getMonth(), now.getDate());
        timeRanges[scope] = ((now - start) / 60000).toString();
    } else {
        timeRanges[scope] = value;
    }
    timeRangeCustom[scope] = { from: '', to: '' };
    persistTimeRanges();
    updateTrLabel(scope);
    closeTimeRangePopup();
    refreshForScope(scope);
}

function applyCustomRange(scope) {
    const from = document.getElementById('tr-from-' + scope).value;
    const to = document.getElementById('tr-to-' + scope).value;
    if (!from) { toast('Select a start date', 'error'); return; }
    timeRangeCustom[scope] = { from, to };
    const fromDate = new Date(from);
    const toDate = to ? new Date(to) : new Date();
    const minutes = Math.ceil((toDate - fromDate) / 60000);
    timeRanges[scope] = minutes.toString();
    persistTimeRanges();
    updateTrLabel(scope);
    closeTimeRangePopup();
    refreshForScope(scope);
}

function updateTrLabel(scope) {
    const label = document.getElementById('tr-label-' + scope);
    const val = timeRanges[scope];
    const cust = timeRangeCustom[scope];
    const trigger = label ? label.closest('.time-range-trigger') : null;
    const isFiltered = !!(val || cust.from);
    if (cust.from) {
        const from = cust.from.replace('T', ' ').slice(5);
        const to = cust.to ? cust.to.replace('T', ' ').slice(5) : 'now';
        label.textContent = from + ' \u2192 ' + to;
    } else {
        const r = quickRanges.find(r => r.value === val);
        label.textContent = r ? r.label : 'All time';
    }
    if (trigger) trigger.classList.toggle('tr-active', isFiltered);
}

function closeTimeRangePopup() {
    document.querySelectorAll('.time-range-popup').forEach(p => p.style.display = 'none');
    activePopup = null;
}

function refreshForScope(scope) {
    if (scope === 'jobs') { jobsPage = 1; fetchJobs(true); }
    else if (scope === 'execs') { allExecsPage = 1; fetchAllExecutions(); }
    else if (scope === 'events') { eventsPage = 1; fetchEvents(); }
}

function shareCurrentPage() {
    let hash = '#/' + currentPage;
    if (currentPage === 'jobs') {
        if (currentJobId) {
            hash = '#/jobs/' + currentJobId;
        } else if (jobsTab !== 'list') {
            hash = '#/jobs/' + jobsTab;
        }
        // Append filter state as query params
        var params = [];
        if (typeof jobSearch !== 'undefined' && jobSearch.statusFilter) params.push('filter=' + encodeURIComponent(jobSearch.statusFilter));
        if (typeof groupFilter !== 'undefined' && groupFilter) params.push('group=' + encodeURIComponent(groupFilter));
        if (typeof jobSearch !== 'undefined' && jobSearch.searchTerm) params.push('search=' + encodeURIComponent(jobSearch.searchTerm));
        if (params.length > 0) hash += '?' + params.join('&');
    } else if (currentPage === 'executions') {
        if (currentExecId) hash = '#/executions/' + currentExecId;
        var ep = [];
        if (typeof execSearch !== 'undefined' && execSearch.statusFilter) ep.push('filter=' + encodeURIComponent(execSearch.statusFilter));
        if (typeof execSearch !== 'undefined' && execSearch.searchTerm) ep.push('search=' + encodeURIComponent(execSearch.searchTerm));
        if (typeof timeRanges !== 'undefined' && timeRanges.execs) ep.push('time=' + timeRanges.execs);
        if (ep.length > 0) hash += '?' + ep.join('&');
    } else if (currentPage === 'events') {
        var evp = [];
        if (typeof eventSearch !== 'undefined' && eventSearch.statusFilter) evp.push('filter=' + encodeURIComponent(eventSearch.statusFilter));
        if (typeof eventSearch !== 'undefined' && eventSearch.searchTerm) evp.push('search=' + encodeURIComponent(eventSearch.searchTerm));
        if (typeof timeRanges !== 'undefined' && timeRanges.events) evp.push('time=' + timeRanges.events);
        if (evp.length > 0) hash += '?' + evp.join('&');
    } else if (currentPage === 'agents') {
        var ap = [];
        if (typeof agentSearch !== 'undefined' && agentSearch.statusFilter) ap.push('filter=' + encodeURIComponent(agentSearch.statusFilter));
        if (typeof agentSearch !== 'undefined' && agentSearch.searchTerm) ap.push('search=' + encodeURIComponent(agentSearch.searchTerm));
        if (ap.length > 0) hash += '?' + ap.join('&');
    }
    var url = window.location.origin + window.location.pathname + hash;
    copyToClipboard(url, 'Link copied to clipboard');
}

function getSinceISO(minutes) {
    if (!minutes) return '';
    const d = new Date(Date.now() - parseInt(minutes) * 60000);
    return d.toISOString();
}

// Close popup on outside click
document.addEventListener('mousedown', function(e) {
    if (activePopup && !e.target.closest('.time-range-wrap')) {
        closeTimeRangePopup();
    }
});
let jobsTotalPages = 1;
let jobsTotal = 0;
let execsPage = 1;
let execsTotalPages = 1;
let execsTotal = 0;
const PER_PAGE = 15;

async function api(method, path, body) {
    const opts = { method, headers: {} };
    const storedKey = localStorage.getItem('kronforce-api-key');
    if (storedKey) {
        opts.headers['Authorization'] = 'Bearer ' + storedKey;
    }
    if (body !== undefined) {
        opts.headers['Content-Type'] = 'application/json';
        opts.body = JSON.stringify(body);
    }
    const res = await fetch(path, opts);
    if (res.status === 401) {
        showLoginScreen();
        throw new Error('Authentication required');
    }
    if (res.status === 403) {
        const err = await res.json().catch(() => ({ error: 'forbidden' }));
        throw new Error(err.error || 'Permission denied');
    }
    if (res.status === 429) {
        const retryAfter = res.headers.get('Retry-After') || '60';
        toast('Rate limited — please wait ' + retryAfter + 's before retrying', 'error');
        throw new Error('Rate limited, retry after ' + retryAfter + ' seconds');
    }
    if (!res.ok) {
        const err = await res.json().catch(() => ({ error: res.statusText }));
        throw new Error(err.error || res.statusText);
    }
    if (res.status === 204) return null;
    return res.json();
}

function toast(msg, type = 'success') {
    // Remove any existing toast
    document.querySelectorAll('.toast').forEach(t => t.remove());
    const el = document.createElement('div');
    el.className = 'toast toast-' + type;
    const icons = { success: '\u2714', error: '\u2718', info: '\u2139' };
    el.innerHTML = '<span>' + (icons[type] || '') + '</span><span>' + esc(msg) + '</span>';
    document.body.appendChild(el);
    setTimeout(() => el.remove(), 4000);
}

// --- Health ---
async function fetchHealth() {
    try {
        await api('GET', '/api/health');
        document.getElementById('health-dot').className = 'health-dot ok';
        document.getElementById('health-text').textContent = 'healthy';
    } catch {
        document.getElementById('health-dot').className = 'health-dot err';
        document.getElementById('health-text').textContent = 'unreachable';
    }
}

// Search/filter factory
function createSearchFilter({ inputId, clearBtnId, filterContainerId, debounceMs, onUpdate }) {
    let debounceTimer = null;
    let statusFilter = '';
    let searchTerm = '';
    return {
        get searchTerm() { return searchTerm; },
        get statusFilter() { return statusFilter; },
        set statusFilter(val) { statusFilter = val; },
        onSearch() {
            const val = document.getElementById(inputId).value;
            document.getElementById(clearBtnId).style.display = val ? '' : 'none';
            clearTimeout(debounceTimer);
            debounceTimer = setTimeout(() => { searchTerm = val.trim().toLowerCase(); onUpdate(); }, debounceMs || 250);
        },
        clearSearch() {
            document.getElementById(inputId).value = '';
            document.getElementById(clearBtnId).style.display = 'none';
            searchTerm = '';
            onUpdate();
        },
        setStatusFilter(btn, status) {
            statusFilter = status;
            document.querySelectorAll('#' + filterContainerId + ' .status-btn').forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            onUpdate();
        }
    };
}

const jobSearch = createSearchFilter({ inputId: 'search-input', clearBtnId: 'search-clear', filterContainerId: 'status-filters', onUpdate: () => fetchJobs(true) });

function emptyState(message, action) {
    let html = '<div class="empty-state"><p>' + message + '</p>';
    if (action && action.label) {
        html += '<button class="btn btn-primary" onclick="' + action.onclick + '">' + action.label + '</button>';
    }
    if (action && action.sub) {
        html += '<p style="font-size:12px;color:var(--text-muted)">' + action.sub + '</p>';
    }
    html += '</div>';
    return html;
}

function openModal(id) {
    document.getElementById(id).style.display = '';
}

function closeModal(id) {
    document.getElementById(id).style.display = 'none';
}

function closeWaitingModal() {
    closeModal('waiting-modal');
}

async function cancelExec() {
    if (!currentExecId) return;
    if (!confirm('Cancel this execution?')) return;
    try {
        await api('POST', '/api/executions/' + currentExecId + '/cancel');
        toast('Cancel request sent');
        closeExecModal();
        if (currentJobId) fetchExecutions(currentJobId);
    } catch (e) {
        toast(e.message, 'error');
    }
}

async function approveExec() {
    if (!currentExecId) return;
    if (!confirm('Approve this execution? The job will run immediately.')) return;
    try {
        await api('POST', '/api/executions/' + currentExecId + '/approve');
        toast('Execution approved');
        closeExecModal();
        fetchAllExecutions();
    } catch (e) {
        toast(e.message, 'error');
    }
}

// --- Formatting ---
function badge(status) {
    return '<span class="badge badge-' + status + '">' + status.replace('_', ' ') + '</span>';
}

function execBadge(status, agentId) {
    if (status === 'pending' && agentId && isCustomAgent(agentId)) {
        return '<span class="badge badge-queued">queued</span>';
    }
    return badge(status);
}

function isCustomAgent(agentId) {
    if (!allAgents || allAgents.length === 0) return false;
    const agent = allAgents.find(a => a.id === agentId);
    return agent && agent.agent_type === 'custom';
}

function fmtTaskBadge(task) {
    if (!task) return '';
    const t = task.type;
    if (t === 'shell') return '<span class="badge badge-enabled">shell</span>';
    if (t === 'http') return '<span class="badge badge-running">http</span>';
    if (t === 'sql') return '<span class="badge badge-paused">sql</span>';
    if (t === 'ftp') return '<span class="badge badge-disabled">ftp</span>';
    if (t === 'script') return '<span class="badge badge-paused">script</span>';
    if (t === 'file_push') return '<span class="badge badge-scheduled">file</span>';
    if (t === 'kafka') return '<span class="badge badge-running">kafka</span>';
    if (t === 'rabbitmq') return '<span class="badge badge-paused">rabbitmq</span>';
    if (t === 'mqtt') return '<span class="badge badge-enabled">mqtt</span>';
    if (t === 'redis') return '<span class="badge badge-disabled">redis</span>';
    if (t === 'kafka_consume') return '<span class="badge badge-running">kafka read</span>';
    if (t === 'mqtt_subscribe') return '<span class="badge badge-enabled">mqtt sub</span>';
    if (t === 'rabbitmq_consume') return '<span class="badge badge-paused">rmq read</span>';
    if (t === 'redis_read') return '<span class="badge badge-disabled">redis read</span>';
    if (t === 'docker_build') return '<span class="badge badge-running">docker</span>';
    return '<span class="badge">' + t + '</span>';
}

function fmtTaskDetail(task) {
    if (!task) return '-';
    if (task.type === 'shell') return fmtTaskBadge(task) + ' <code>' + esc(task.command) + '</code>';
    if (task.type === 'http') return fmtTaskBadge(task) + ' <code>' + esc(task.method.toUpperCase() + ' ' + task.url) + '</code>';
    if (task.type === 'sql') return fmtTaskBadge(task) + ' <code>' + esc(task.driver + ': ' + task.query.slice(0, 80)) + '</code>';
    if (task.type === 'ftp') return fmtTaskBadge(task) + ' <code>' + esc(task.direction + ' ' + task.protocol + '://' + task.host + task.remote_path) + '</code>';
    if (task.type === 'script') return fmtTaskBadge(task) + ' <span class="job-name" onclick="showPage(\'scripts\')" style="cursor:pointer">' + esc(task.script_name) + '</span>';
    if (task.type === 'file_push') {
        const size = task.content_base64 ? (Math.floor(task.content_base64.length * 3 / 4 / 1024 * 10) / 10) + ' KB' : '?';
        return fmtTaskBadge(task) + ' <code>' + esc(task.filename) + ' &rarr; ' + esc(task.destination) + '</code> <span class="time-text">(' + size + ')</span>';
    }
    if (task.type === 'kafka') return fmtTaskBadge(task) + ' <code>' + esc(task.broker) + ' / ' + esc(task.topic) + '</code>';
    if (task.type === 'rabbitmq') return fmtTaskBadge(task) + ' <code>' + esc(task.exchange) + ' / ' + esc(task.routing_key) + '</code>';
    if (task.type === 'mqtt') return fmtTaskBadge(task) + ' <code>' + esc(task.broker) + ':' + (task.port || 1883) + ' / ' + esc(task.topic) + '</code>';
    if (task.type === 'redis') return fmtTaskBadge(task) + ' <code>' + esc(task.channel) + '</code>';
    return task.type;
}

// Cache agent names so deleted agents can still be displayed
const agentNameCache = {};

function cacheAgentNames() {
    for (const a of allAgents) {
        agentNameCache[a.id] = a.name;
    }
}

function fmtAgentLink(agentId) {
    const agent = allAgents.find(a => a.id === agentId);
    const name = agent ? agent.name : (agentNameCache[agentId] || agentId.slice(0, 8));
    if (agent) agentNameCache[agentId] = agent.name;
    const isCustom = agent && agent.agent_type === 'custom';
    const badgeClass = isCustom ? 'badge-paused' : 'badge-running';
    const icon = isCustom ? '&#9881;' : '&#128421;';
    return '<span class="badge ' + badgeClass + '" style="cursor:pointer" onclick="showPage(\'agents\')" title="' + (isCustom ? 'Custom agent' : 'Standard agent') + '">' + icon + ' ' + esc(name) + '</span>';
}

function fmtTarget(t) {
    if (!t || t.type === 'local') return '<span class="badge badge-active">controller</span>';
    if (t.type === 'agent') return fmtAgentLink(t.agent_id);
    if (t.type === 'any') return '<span class="badge badge-running">any agent</span>';
    if (t.type === 'all') return '<span class="badge badge-paused">all agents</span>';
    return t.type;
}

function fmtScheduleDetail(s) {
    if (s.type === 'event' && s.value) {
        let html = '<span class="badge badge-running">\u26A1 event trigger</span> ';
        html += '<code>' + esc(s.value.kind_pattern) + '</code>';
        if (s.value.severity) html += ' <span class="badge badge-' + s.value.severity + '">' + s.value.severity + '</span>';
        if (s.value.job_name_filter) html += ' <span class="time-text">filter: ' + esc(s.value.job_name_filter) + '</span>';
        return html;
    }
    return fmtSchedule(s);
}

function fmtSchedule(s) {
    if (s.type === 'cron') return describeCron(s.value);
    if (s.type === 'one_shot') return 'once: ' + fmtDate(s.value);
    if (s.type === 'event' && s.value) {
        let desc = 'on ' + s.value.kind_pattern;
        if (s.value.severity) desc += ' (' + s.value.severity + ')';
        if (s.value.job_name_filter) desc += ' [' + s.value.job_name_filter + ']';
        return desc;
    }
    if (s.type === 'calendar' && s.value) {
        let desc = s.value.anchor.replace(/_/g, ' ');
        if (s.value.anchor === 'nth_weekday' && s.value.weekday) {
            const ordinal = {1:'1st',2:'2nd',3:'3rd',4:'4th'}[s.value.nth] || s.value.nth + 'th';
            desc = ordinal + ' ' + s.value.weekday;
        }
        if (s.value.offset_days > 0) desc += ' +' + s.value.offset_days + 'd';
        else if (s.value.offset_days < 0) desc += ' ' + s.value.offset_days + 'd';
        desc += ' @ ' + String(s.value.hour || 0).padStart(2, '0') + ':' + String(s.value.minute || 0).padStart(2, '0');
        return desc;
    }
    return 'on-demand';
}

function describeCron(expr) {
    const parts = expr.split(/\s+/);
    if (parts.length !== 6) return expr;
    const [sec, min, hr, dom, mon, dow] = parts;
    const pad = n => String(n).padStart(2, '0');
    const dayNames = {0:'Sun',1:'Mon',2:'Tue',3:'Wed',4:'Thu',5:'Fri',6:'Sat'};

    // Every N seconds
    if (sec.startsWith('*/')) return 'every ' + sec.slice(2) + 's';
    if (sec.includes('/')) return 'every ' + sec.split('/')[1] + 's';
    if (sec === '*') return 'every second';

    // Every N minutes
    if (min.startsWith('*/')) return 'every ' + min.slice(2) + ' min';
    if (min.includes('/')) return 'every ' + min.split('/')[1] + ' min';
    if (min === '*' && hr === '*' && dom === '*') return 'every minute';

    // Every N hours
    if (hr.startsWith('*/')) return 'every ' + hr.slice(2) + 'h';
    if (hr.includes('/')) return 'every ' + hr.split('/')[1] + 'h';
    if (hr === '*' && min !== '*' && dom === '*') return 'hourly at :' + pad(parseInt(min));

    // Weekly
    if (dow !== '*' && dom === '*') {
        const days = dow.split(',').map(d => dayNames[d.trim()] || d).join(', ');
        if (hr !== '*') return days + ' at ' + pad(parseInt(hr)) + ':' + pad(parseInt(min));
        return 'weekly on ' + days;
    }

    // Monthly
    if (dom !== '*' && !dom.startsWith('*/') && mon === '*' && dow === '*') {
        if (hr !== '*') return 'monthly day ' + dom + ' at ' + pad(parseInt(hr)) + ':' + pad(parseInt(min));
        return 'monthly on day ' + dom;
    }

    // Daily / every N days
    if (dom.startsWith('*/')) {
        return 'every ' + dom.slice(2) + ' days at ' + pad(parseInt(hr)) + ':' + pad(parseInt(min));
    }
    if (hr !== '*' && min !== '*' && dom === '*' && dow === '*') {
        return 'daily at ' + pad(parseInt(hr)) + ':' + pad(parseInt(min));
    }

    // Fallback
    return expr;
}

function fmtDate(iso) {
    if (!iso) return '-';
    const d = new Date(iso);
    const now = new Date();
    const diff = now - d;
    const pad = n => String(n).padStart(2, '0');
    const utc = d.getUTCFullYear() + '-' + pad(d.getUTCMonth() + 1) + '-' + pad(d.getUTCDate()) + ' ' + pad(d.getUTCHours()) + ':' + pad(d.getUTCMinutes()) + ':' + pad(d.getUTCSeconds()) + ' UTC';
    // Show relative + UTC tooltip
    let relative;
    if (diff >= 0 && diff < 60000) relative = Math.floor(diff / 1000) + 's ago';
    else if (diff >= 0 && diff < 3600000) relative = Math.floor(diff / 60000) + 'm ago';
    else if (diff >= 0 && diff < 86400000) relative = Math.floor(diff / 3600000) + 'h ago';
    else if (diff < 0 && diff > -86400000) relative = 'in ' + Math.floor(-diff / 60000) + 'm';
    else relative = utc;
    return '<span title="' + utc + '">' + relative + '</span>';
}

function fmtDateUTC(iso) {
    if (!iso) return '-';
    const d = new Date(iso);
    const pad = n => String(n).padStart(2, '0');
    return d.getUTCFullYear() + '-' + pad(d.getUTCMonth() + 1) + '-' + pad(d.getUTCDate()) + ' ' + pad(d.getUTCHours()) + ':' + pad(d.getUTCMinutes()) + ':' + pad(d.getUTCSeconds()) + ' UTC';
}

function toLocalDatetimeString(d) {
    const pad = n => String(n).padStart(2, '0');
    return d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate()) + 'T' + pad(d.getHours()) + ':' + pad(d.getMinutes());
}

function fmtSeconds(secs) {
    if (secs < 60) return secs + 's';
    if (secs < 3600) return Math.floor(secs / 60) + 'm';
    if (secs < 86400) return Math.floor(secs / 3600) + 'h';
    return Math.floor(secs / 86400) + 'd';
}

function fmtDuration(start, end) {
    if (!start) return '-';
    if (!end) return 'running...';
    const ms = new Date(end) - new Date(start);
    if (ms < 1000) return ms + 'ms';
    if (ms < 60000) return (ms / 1000).toFixed(1) + 's';
    return Math.floor(ms / 60000) + 'm ' + Math.floor((ms % 60000) / 1000) + 's';
}

function fmtLastRun(exec) {
    if (!exec) return '<span class="run-indicator neutral"><span class="dot"></span>never run</span>';
    const s = exec.status;
    let cls = 'neutral';
    let label = s.replace('_', ' ');
    if (s === 'succeeded') { cls = 'success'; }
    else if (s === 'failed' || s === 'timed_out') { cls = 'failure'; }
    else if (s === 'running') { cls = 'running'; }
    const clickable = exec.id ? ' style="cursor:pointer" onclick="showExecDetail(\'' + exec.id + '\')" title="View execution details"' : '';
    let html = '<div class="last-run"' + clickable + '><span class="run-indicator ' + cls + '"><span class="dot"></span>' + label + '</span>';
    if (exec.finished_at) {
        html += '<span class="last-run-time">' + fmtDate(exec.finished_at) + '</span>';
    }
    html += '</div>';
    return html;
}

function fmtCounts(counts, jobId) {
    if (!counts || counts.total === 0) return '<span class="time-text">-</span>';
    let html = '<div class="exec-counts">';
    html += '<span class="exec-count success" title="Succeeded">\u2714 ' + counts.succeeded + '</span>';
    if (counts.failed > 0) {
        const onclick = jobId ? ' style="cursor:pointer" onclick="showJobDetail(\'' + jobId + '\');setTimeout(function(){setDetailTab(\'history\')},100)" title="View failed executions"' : '';
        html += '<span class="exec-count fail"' + onclick + '>\u2718 ' + counts.failed + '</span>';
    }
    html += '<span class="exec-count total" title="Total runs">\u2211 ' + counts.total + '</span>';
    html += '</div>';
    return html;
}

function fmtTrigger(t) {
    if (t.type === 'scheduler') return 'scheduler';
    if (t.type === 'api') return 'api';
    if (t.type === 'dependency') return 'dep';
    if (t.type === 'event') return '\u26A1 event';
    return t.type;
}

function esc(s) {
    const d = document.createElement('div');
    d.textContent = s;
    return d.innerHTML;
}

// --- Pagination ---
function renderPagination(containerId, currentPage, totalPages, total, goToFn) {
    const el = document.getElementById(containerId);
    if (totalPages <= 1) {
        el.innerHTML = total > 0 ? '<span class="pagination-info">' + total + ' total</span>' : '';
        return;
    }
    const fnName = '_pag_' + containerId.replace(/-/g, '_');
    window[fnName] = goToFn;
    let html = '<span class="pagination-info">Page ' + currentPage + ' of ' + totalPages + ' (' + total + ' total)</span>';
    html += '<div class="pagination-controls">';
    html += '<button class="page-btn" ' + (currentPage <= 1 ? 'disabled' : 'onclick="' + fnName + '(' + (currentPage - 1) + ')"') + '>&laquo;</button>';
    const start = Math.max(1, currentPage - 2);
    const end = Math.min(totalPages, currentPage + 2);
    if (start > 1) html += '<button class="page-btn" onclick="' + fnName + '(1)">1</button>';
    if (start > 2) html += '<span style="color:var(--text-muted)">...</span>';
    for (let i = start; i <= end; i++) {
        html += '<button class="page-btn' + (i === currentPage ? ' active' : '') + '" onclick="' + fnName + '(' + i + ')">' + i + '</button>';
    }
    if (end < totalPages - 1) html += '<span style="color:var(--text-muted)">...</span>';
    if (end < totalPages) html += '<button class="page-btn" onclick="' + fnName + '(' + totalPages + ')">' + totalPages + '</button>';
    html += '<button class="page-btn" ' + (currentPage >= totalPages ? 'disabled' : 'onclick="' + fnName + '(' + (currentPage + 1) + ')"') + '>&raquo;</button>';
    html += '</div>';
    el.innerHTML = html;
}

// --- Page Navigation ---
let currentPage = 'jobs';

function toggleWorkMenu() {
    const menu = document.getElementById('work-submenu');
    menu.classList.toggle('open');
}

function toggleCodeMenu() {
    const menu = document.getElementById('code-submenu');
    menu.classList.toggle('open');
}

function toggleManageMenu() {
    const menu = document.getElementById('manage-submenu');
    menu.classList.toggle('open');
}

function showPage(page) {
    currentPage = page;
    document.querySelectorAll('.nav-tab').forEach(t => t.classList.remove('active'));
    document.querySelectorAll('.nav-submenu-item').forEach(t => t.classList.remove('active'));

    const tab = document.getElementById('tab-' + page);
    if (tab) {
        tab.classList.add('active');
        if (tab.classList.contains('nav-submenu-item')) {
            // Submenu item — also highlight parent button
            const group = tab.closest('.nav-tab-group');
            if (group) {
                const parentBtn = group.querySelector('.nav-tab');
                if (parentBtn) parentBtn.classList.add('active');
            }
        }
    }

    for (const v of ALL_VIEWS) {
        document.getElementById(v + '-view').style.display = v === page ? '' : 'none';
    }
    for (const [p, barId] of Object.entries(VIEW_ACTION_BARS)) {
        document.getElementById(barId).style.display = p === page ? '' : 'none';
    }

    if (page === 'dashboard') {
        renderDashboard();
    } else if (page === 'agents') {
        fetchAgents();
    } else if (page === 'groups' || page === 'map') {
        // Redirect old URLs to jobs with map tab
        jobsTab = 'map';
        showPage('jobs');
        return;
    } else if (page === 'jobs') {
        currentJobId = null;
        fetchGroups();
        // Sync filter button state with current filter value
        document.querySelectorAll('#status-filters .status-btn').forEach(b => {
            b.classList.toggle('active', b.dataset.status === jobSearch.statusFilter || (!jobSearch.statusFilter && b.dataset.status === ''));
        });
        // Sync group picker label
        var gpLabel = document.getElementById('group-picker-label');
        if (gpLabel) gpLabel.textContent = groupFilter || 'All Groups';
        var gpBtn = document.getElementById('group-picker-btn');
        if (gpBtn) gpBtn.classList.toggle('group-picker-active', !!groupFilter);
        setJobsTab(jobsTab);
    } else if (page === 'executions') {
        // Sync exec filter button state
        document.querySelectorAll('#exec-status-filters .status-btn').forEach(b => {
            const onclick = b.getAttribute('onclick') || '';
            const isMatch = execSearch.statusFilter ? onclick.includes("'" + execSearch.statusFilter + "'") : onclick.includes("''");
            b.classList.toggle('active', isMatch);
        });
        fetchAllExecutions();
    } else if (page === 'scripts') {
        fetchScripts();
    } else if (page === 'events') {
        fetchEvents();
    } else if (page === 'variables') {
        fetchVariables();
    } else if (page === 'connections') {
        if (typeof fetchConnections === 'function') fetchConnections();
    } else if (page === 'settings') {
        updateThemeButtons();
        renderSettingsAuth();
        loadRetention();
        loadNotificationSettings();
        showSettingsTab(currentSettingsTab || 'general');
    }
}

function scrollToDocTopic(topicId) {
    const container = document.getElementById('docs-content');
    const el = document.getElementById(topicId);
    if (container && el) {
        container.scrollTo({ top: el.offsetTop - container.offsetTop, behavior: 'smooth' });
    }
    document.querySelectorAll('.docs-topic').forEach(t => t.classList.remove('active'));
    const btn = document.querySelector('.docs-topic[onclick*="' + topicId + '"]');
    if (btn) btn.classList.add('active');
}

// --- Auto-Refresh ---
let autoRefreshEnabled = true;
let refreshIntervalSecs = 5;
let countdownRemaining = 5;
let countdownTimer = null;

function toggleAutoRefresh() {
    autoRefreshEnabled = !autoRefreshEnabled;
    const btn = document.getElementById('refresh-toggle');
    const label = document.getElementById('refresh-label');
    if (autoRefreshEnabled) {
        btn.classList.add('active');
        label.textContent = 'On';
        startPolling();
    } else {
        btn.classList.remove('active');
        label.textContent = 'Off';
        stopPolling();
        document.getElementById('refresh-countdown').textContent = '';
    }
}

function changeRefreshInterval() {
    refreshIntervalSecs = parseInt(document.getElementById('refresh-interval').value);
    if (autoRefreshEnabled) {
        startPolling();
    }
}

async function refreshNow() {
    const btn = document.getElementById('refresh-toggle');
    btn.classList.add('spinning');
    fetchHealth();
    try { const a = await api('GET', '/api/agents'); allAgents = a; cacheAgentNames(); } catch(e) {}
    if (currentPage === 'dashboard') {
        await renderDashboard();
    } else if (currentPage === 'agents') {
        renderAgents();
    } else if (currentPage === 'executions') {
        await fetchAllExecutions();
    } else if (currentPage === 'events') {
        await fetchEvents();
    } else if (currentJobId) {
        await fetchExecutions(currentJobId);
    } else {
        await fetchJobs();
    }
    setTimeout(() => btn.classList.remove('spinning'), 600);
    if (autoRefreshEnabled) {
        countdownRemaining = refreshIntervalSecs;
    }
}

function startPolling() {
    stopPolling();
    countdownRemaining = refreshIntervalSecs;
    updateCountdown();
    countdownTimer = setInterval(() => {
        countdownRemaining--;
        if (countdownRemaining <= 0) {
            doRefreshTick();
            countdownRemaining = refreshIntervalSecs;
        }
        updateCountdown();
    }, 1000);
}

async function doRefreshTick() {
    const btn = document.getElementById('refresh-toggle');
    btn.classList.add('spinning');
    fetchHealth();
    // Always refresh agent names cache
    try { const a = await api('GET', '/api/agents'); allAgents = a; cacheAgentNames(); } catch(e) {}
    if (currentPage === 'dashboard') {
        await renderDashboard();
    } else if (currentPage === 'agents') {
        renderAgents();
    } else if (currentPage === 'executions') {
        await fetchAllExecutions();
    } else if (currentPage === 'events') {
        await fetchEvents();
    } else if (currentJobId) {
        await fetchExecutions(currentJobId);
    } else {
        await fetchJobs();
    }
    setTimeout(() => btn.classList.remove('spinning'), 600);
}

function stopPolling() {
    if (countdownTimer) { clearInterval(countdownTimer); countdownTimer = null; }
}

function updateCountdown() {
    document.getElementById('refresh-countdown').textContent = countdownRemaining + 's';
}

// --- Auth ---
let currentUser = null;

function showLoginScreen() {
    document.getElementById('login-screen').style.display = '';
    document.getElementById('app-layout').style.display = 'none';
    document.getElementById('login-key').focus();
    // Check if OIDC is configured and show SSO button
    checkOidcConfig();
}

function showApp() {
    document.getElementById('login-screen').style.display = 'none';
    document.getElementById('app-layout').style.display = '';
}

function showDemoBanner() {
    if (!currentUser || currentUser.auth_type !== 'demo') return;
    if (document.getElementById('demo-banner')) return;
    const banner = document.createElement('div');
    banner.id = 'demo-banner';
    banner.style.cssText = 'position:fixed;top:0;left:0;right:0;z-index:9999;background:var(--accent);color:#fff;text-align:center;padding:8px 16px;font-size:13px;font-weight:500;display:flex;align-items:center;justify-content:center;gap:12px';
    banner.innerHTML = 'You are viewing a <strong>read-only demo</strong> of Kronforce. Explore freely — nothing you do will break anything.' +
        ' <a href="https://kronforce.dev" style="color:#fff;text-decoration:underline;font-weight:600" target="_blank">Learn more</a>' +
        '<button onclick="this.parentElement.remove();document.getElementById(\'app-layout\').style.marginTop=\'\'" style="background:none;border:none;color:#fff;cursor:pointer;font-size:18px;margin-left:8px;padding:0 4px;opacity:0.7">&times;</button>';
    document.body.prepend(banner);
    document.getElementById('app-layout').style.marginTop = '38px';
}

async function checkOidcConfig() {
    try {
        const resp = await fetch('/api/auth/oidc/config');
        if (resp.ok) {
            const data = await resp.json();
            const ssoDiv = document.getElementById('sso-login');
            if (ssoDiv) ssoDiv.style.display = data.enabled ? '' : 'none';
        }
    } catch (e) { /* ignore */ }
}

function doSsoLogin() {
    window.location.href = '/api/auth/oidc/login';
}

async function doLogin() {
    const key = document.getElementById('login-key').value.trim();
    if (!key) return;
    localStorage.setItem('kronforce-api-key', key);
    try {
        currentUser = await api('GET', '/api/auth/me');
        document.getElementById('login-error').textContent = '';
        showApp();
        renderSidebarUser();
        handleRoute();
    } catch (e) {
        document.getElementById('login-error').textContent = 'Invalid API key';
        localStorage.removeItem('kronforce-api-key');
    }
}

async function doLogout() {
    // If OIDC session, clear server-side session first
    if (currentUser && currentUser.auth_type === 'oidc') {
        try { await fetch('/api/auth/logout', { method: 'POST' }); } catch (e) { /* ignore */ }
    }
    localStorage.removeItem('kronforce-api-key');
    currentUser = null;
    showLoginScreen();
}

async function checkAuth() {
    try {
        currentUser = await api('GET', '/api/auth/me');
        showApp();
        renderSidebarUser();
        return true;
    } catch (e) {
        // If 401, login screen is already shown by the api() function
        return false;
    }
}

function renderSidebarUser() {
    const el = document.getElementById('sidebar-user');
    if (!el) return;
    if (currentUser && currentUser.authenticated) {
        el.innerHTML = '<span class="sidebar-user-name">' + esc(currentUser.name) + '</span>' +
            '<span class="sidebar-user-role">' + esc(currentUser.role) + '</span>';
    } else {
        el.innerHTML = '';
    }
}

// --- Key Management (Settings) ---

function renderSettingsAuth() {
    const info = document.getElementById('auth-info');
    if (currentUser && currentUser.authenticated) {
        info.innerHTML =
            '<div style="font-size:13px">Signed in as <strong>' + esc(currentUser.name) + '</strong></div>' +
            '<div style="font-size:12px;color:var(--text-secondary);margin-top:4px">Role: ' + badge(currentUser.role) + ' &middot; Key: <span class="key-prefix">' + esc(currentUser.key_prefix) + '...</span></div>';
        // Show keys card for admins
        document.getElementById('keys-card').style.display = currentUser.role === 'admin' ? '' : 'none';
        if (currentUser.role === 'admin') fetchKeys();
    } else {
        info.innerHTML = '<div style="font-size:13px;color:var(--text-muted)">No API keys configured. Authentication is disabled.</div>';
        document.getElementById('keys-card').style.display = 'none';
    }
}

// --- Theme ---
let currentTheme = localStorage.getItem('kronforce-theme') || 'dark';

function clearBrowserCache() {
    if (!confirm('Clear all browser cache for this site and reload?')) return;
    // Clear localStorage
    localStorage.clear();
    // Clear sessionStorage
    sessionStorage.clear();
    // Unregister service workers
    if ('serviceWorker' in navigator) {
        navigator.serviceWorker.getRegistrations().then(function(registrations) {
            for (const reg of registrations) reg.unregister();
        });
    }
    // Clear caches API
    if ('caches' in window) {
        caches.keys().then(function(names) {
            for (const name of names) caches.delete(name);
        });
    }
    // Hard reload
    setTimeout(function() { location.reload(true); }, 200);
}

function setTheme(theme) {
    currentTheme = theme;
    localStorage.setItem('kronforce-theme', theme);
    applyTheme();
    updateThemeButtons();
}

function applyTheme() {
    let resolved = currentTheme;
    if (resolved === 'system') {
        resolved = window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
    }
    if (resolved === 'light') {
        document.documentElement.setAttribute('data-theme', 'light');
    } else {
        document.documentElement.removeAttribute('data-theme');
    }
}

function updateThemeButtons() {
    document.querySelectorAll('.theme-btn').forEach(b => b.classList.remove('active'));
    const btn = document.getElementById('theme-' + currentTheme);
    if (btn) btn.classList.add('active');
}

// Listen for system theme changes
window.matchMedia('(prefers-color-scheme: light)').addEventListener('change', () => {
    if (currentTheme === 'system') applyTheme();
});

// Apply on load
applyTheme();

// --- Routing ---

function updateHash() {
    let newHash;
    if (currentJobId) {
        newHash = '#/jobs/' + currentJobId;
    } else if (currentExecId && currentPage === 'executions') {
        newHash = '#/executions/' + currentExecId;
    } else {
        newHash = '#/' + currentPage;
        // Persist filter state in the hash for jobs page
        if (currentPage === 'jobs') {
            if (typeof jobsTab !== 'undefined' && jobsTab !== 'list') {
                newHash = '#/jobs/' + jobsTab;
            }
            var params = [];
            if (typeof jobSearch !== 'undefined' && jobSearch.statusFilter) params.push('filter=' + encodeURIComponent(jobSearch.statusFilter));
            if (typeof groupFilter !== 'undefined' && groupFilter) params.push('group=' + encodeURIComponent(groupFilter));
            if (typeof jobSearch !== 'undefined' && jobSearch.searchTerm) params.push('search=' + encodeURIComponent(jobSearch.searchTerm));
            if (params.length > 0) newHash += '?' + params.join('&');
        }
    }
    if (location.hash !== newHash) {
        history.replaceState(null, '', newHash);
    }
}

function parseHashParams(hash) {
    var qIdx = hash.indexOf('?');
    if (qIdx === -1) return {};
    var params = {};
    hash.slice(qIdx + 1).split('&').forEach(function(p) {
        var kv = p.split('=');
        if (kv.length === 2) params[kv[0]] = decodeURIComponent(kv[1]);
    });
    return params;
}

function handleRoute() {
    const hash = location.hash || '#/dashboard';
    const cleanHash = hash.split('?')[0];
    const params = parseHashParams(hash);
    const parts = cleanHash.replace('#/', '').split('/');

    // Apply shared filter state from URL params (set variables only, don't trigger renders)
    if (parts[0] === 'jobs') {
        if (params.filter) jobSearch.statusFilter = params.filter;
        if (params.group) groupFilter = params.group;
        if (params.search) {
            // Will be applied after DOM renders via setTimeout below
        }
    }
    if (parts[0] === 'executions' && typeof execSearch !== 'undefined') {
        if (params.filter) execSearch.statusFilter = params.filter;
        if (params.search) {
            var esi = document.getElementById('exec-search-input');
            if (esi) esi.value = params.search;
        }
        if (params.time && typeof timeRanges !== 'undefined') timeRanges.execs = params.time;
    }
    if (parts[0] === 'events' && typeof eventSearch !== 'undefined') {
        if (params.filter) eventSearch.statusFilter = params.filter;
        if (params.search) {
            var evi = document.getElementById('event-search-input');
            if (evi) evi.value = params.search;
        }
        if (params.time && typeof timeRanges !== 'undefined') timeRanges.events = params.time;
    }
    if (parts[0] === 'agents' && typeof agentSearch !== 'undefined') {
        if (params.filter) agentSearch.statusFilter = params.filter;
        if (params.search) {
            var asi = document.getElementById('agent-search-input');
            if (asi) asi.value = params.search;
        }
    }

    if (parts[0] === 'jobs' && parts[1]) {
        // Jobs tab: #/jobs/stages, #/jobs/map
        if (['groups', 'stages', 'map'].includes(parts[1])) {
            jobsTab = parts[1];
            showPage('jobs');
            return;
        }
        // Job detail: #/jobs/{id}
        currentPage = 'jobs';
        document.querySelectorAll('.nav-tab').forEach(t => t.classList.remove('active'));
        document.getElementById('tab-jobs').classList.add('active');
        showJobDetail(parts[1]);
        return;
    }

    if (parts[0] === 'executions' && parts[1]) {
        // Execution detail: #/executions/{id}
        currentPage = 'executions';
        document.querySelectorAll('.nav-tab').forEach(t => t.classList.remove('active'));
        document.getElementById('tab-executions').classList.add('active');
        showPage('executions');
        showExecDetail(parts[1]);
        return;
    }

    const page = parts[0] || 'jobs';
    if (['dashboard', 'jobs', 'groups', 'map', 'agents', 'executions', 'scripts', 'events', 'variables', 'docs', 'guide', 'settings'].includes(page)) {
        showPage(page);
    } else {
        showPage('jobs');
    }

    // Sync UI elements after page renders
    setTimeout(function() {
        if (params.group && parts[0] === 'jobs') {
            var label = document.getElementById('group-picker-label');
            if (label) label.textContent = params.group;
            var btn = document.getElementById('group-picker-btn');
            if (btn) btn.classList.toggle('group-picker-active', true);
        }
        if (params.search && parts[0] === 'jobs') {
            var si = document.getElementById('search-input');
            if (si) si.value = params.search;
        }
        if (params.search && parts[0] === 'executions') {
            var esi = document.getElementById('exec-search-input');
            if (esi) esi.value = params.search;
        }
        if (params.search && parts[0] === 'events') {
            var evi = document.getElementById('event-search-input');
            if (evi) evi.value = params.search;
        }
        if (params.search && parts[0] === 'agents') {
            var asi = document.getElementById('agent-search-input');
            if (asi) asi.value = params.search;
        }
    }, 100);
}

// Patch showPage and showJobDetail to update the hash
const _origShowPage = showPage;
showPage = function(page) {
    _origShowPage(page);
    updateHash();
};

const _origShowJobDetail = showJobDetail;
showJobDetail = async function(id) {
    await _origShowJobDetail(id);
    updateHash();
};

const _origShowJobsList = showJobsList;
showJobsList = function() {
    _origShowJobsList();
    updateHash();
};

window.addEventListener('hashchange', handleRoute);
window.addEventListener('popstate', handleRoute);

// --- Rich Empty States ---
function renderRichEmptyState(config) {
    let html = '<div class="rich-empty">';
    if (config.icon) html += '<div class="rich-empty-icon">' + config.icon + '</div>';
    html += '<div class="rich-empty-title">' + esc(config.title) + '</div>';
    if (config.description) html += '<div class="rich-empty-desc">' + config.description + '</div>';
    if (config.actions && config.actions.length > 0) {
        html += '<div class="rich-empty-actions">';
        for (const a of config.actions) {
            const cls = a.primary ? 'btn btn-primary btn-sm' : 'btn btn-ghost btn-sm';
            html += '<button class="' + cls + '" onclick="' + a.onclick + '">' + esc(a.label) + '</button>';
        }
        html += '</div>';
    }
    if (config.hint) html += '<div class="rich-empty-hint">' + config.hint + '</div>';
    html += '</div>';
    return html;
}

// --- Init ---
// Fill guide controller URL
(function() {
    const el = document.getElementById('guide-controller-url');
    if (el) el.textContent = getControllerUrl();
})();
fetchHealth();
(async () => {
    // Check demo mode — skip login if enabled
    try {
        const cfg = await (await fetch('/api/config')).json();
        if (cfg.ai_enabled && typeof checkAiEnabled === 'function') {
            aiEnabled = true;
        }
        if (cfg.demo_mode) {
            currentUser = { authenticated: true, auth_type: 'demo', name: 'Demo', role: 'viewer' };
            showApp();
            renderSidebarUser();
            await fetchAgents();
            for (const scope of ['jobs', 'execs', 'events']) {
                if (timeRanges[scope]) updateTrLabel(scope);
            }
            handleRoute();
            startPolling();
            if (typeof maybeStartTour === 'function') maybeStartTour();
            return;
        }
    } catch (e) { /* not demo mode */ }

    const authed = await checkAuth();
    if (authed) {
        await fetchAgents();
        for (const scope of ['jobs', 'execs', 'events']) {
            if (timeRanges[scope]) updateTrLabel(scope);
        }
        handleRoute();
        startPolling();
        if (typeof maybeStartTour === 'function') maybeStartTour();
    }
})();
