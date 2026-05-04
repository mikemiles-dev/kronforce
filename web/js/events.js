// Kronforce - Event log
// --- Events ---
let eventsPage = 1;
const eventSearch = createSearchFilter({ inputId: 'event-search-input', clearBtnId: 'event-search-clear', filterContainerId: 'event-type-filters', onUpdate: () => { eventsPage = 1; fetchEvents(); } });

async function fetchEvents() {
    try {
        const fetchSize = (eventSearch.searchTerm || eventSearch.statusFilter) ? 200 : 50;
        let eventsQs = '?page=' + eventsPage + '&per_page=' + fetchSize;
        if (timeRanges.events) eventsQs += '&since=' + encodeURIComponent(getSinceISO(timeRanges.events));
        const res = await api('GET', '/api/events' + eventsQs);
        let events = res.data;

        if (eventSearch.statusFilter) {
            events = events.filter(e => e.severity === eventSearch.statusFilter);
        }
        if (eventSearch.searchTerm) {
            events = events.filter(e =>
                e.message.toLowerCase().includes(eventSearch.searchTerm) ||
                e.kind.toLowerCase().includes(eventSearch.searchTerm) ||
                (e.api_key_name && e.api_key_name.toLowerCase().includes(eventSearch.searchTerm))
            );
        }

        renderEvents(events);
        const total = (eventSearch.searchTerm || eventSearch.statusFilter) ? events.length : res.total;
        const pages = (eventSearch.searchTerm || eventSearch.statusFilter) ? 1 : res.total_pages;
        renderPagination('events-pagination', eventsPage, pages, total, goToEventsPage);
    } catch (e) {
        console.error('fetchEvents:', e);
    }
}

function goToEventsPage(p) {
    eventsPage = p;
    fetchEvents();
}

function renderEvents(events) {
    const wrap = document.getElementById('events-list-wrap');
    if (events.length === 0) {
        const hasFilters = eventSearch.statusFilter || eventSearch.searchTerm || timeRanges.events;
        if (hasFilters) {
            wrap.innerHTML = renderRichEmptyState({
                icon: '&#128270;',
                title: 'No matching events',
                description: 'No events match the current filters.',
                actions: [
                    { label: 'Clear Filters', onclick: "eventSearch.setStatusFilter(document.querySelector('#event-type-filters .status-btn'), '');fetchEvents()", primary: true },
                ],
            });
        } else {
            wrap.innerHTML = renderRichEmptyState({
                icon: '&#128276;',
                title: 'No events yet',
                description: 'Events are logged when jobs run, agents change status, API keys are managed, and output patterns match. Activity will appear here automatically.',
                actions: [
                    { label: 'Create a Job', onclick: "openCreateModal()", primary: true },
                ],
            });
        }
        return;
    }

    let html = '<div class="event-timeline">';
    for (const e of events) {
        const icon = eventIcon(e.severity, e.kind);
        const jobLink = e.job_id ? '<span class="event-link" onclick="showJobDetail(\'' + e.job_id + '\')" title="Open job">&#128202; ' + resolveJobName(e.job_id) + '</span>' : '';
        const agentLink = e.agent_id ? '<span class="event-link" onclick="showPage(\'settings\');showSettingsTab(\'agents\')" title="Open agents settings">&#128421; ' + resolveAgentName(e.agent_id) + '</span>' : '';
        const keyLink = e.api_key_name ? '<span class="event-link" onclick="showPage(\'settings\');showSettingsTab(\'auth\')" title="Open API keys settings">&#128100; ' + esc(e.api_key_name) + '</span>' : '';
        const execLink = e.execution_id ? '<span class="event-link" onclick="showExecDetail(\'' + e.execution_id + '\')" title="View execution output">&#128196; output</span>' : '';

        html += '<div class="event-item">';
        html += '<div class="event-icon ' + e.severity + '">' + icon + '</div>';
        html += '<div class="event-body">';
        html += '<div class="event-message">' + esc(e.message) + '</div>';
        html += '<div class="event-meta">';
        html += '<span class="event-kind">' + e.kind + '</span>';
        if (keyLink) html += keyLink;
        if (jobLink) html += jobLink;
        if (agentLink) html += agentLink;
        if (execLink) html += execLink;
        if (e.details) html += '<span class="event-kind" title="' + esc(e.details) + '">details</span>';
        html += '</div>';
        html += '</div>';
        html += '<div class="event-time">' + fmtDate(e.timestamp) + '</div>';
        html += '</div>';
    }
    html += '</div>';
    wrap.innerHTML = html;
}

function eventIcon(severity, kind) {
    if (kind.startsWith('agent')) return '\uD83D\uDDA5';
    if (kind === 'job.created') return '+';
    if (kind === 'job.deleted') return '\uD83D\uDDD1';
    if (kind === 'job.triggered') return '\u25B6';
    if (severity === 'success') return '\u2714';
    if (severity === 'error') return '\u2718';
    if (severity === 'warning') return '\u26A0';
    return '\u2139';
}

function resolveJobName(jobId) {
    const j = allJobs.find(j => j.id === jobId);
    return j ? j.name : jobId.slice(0, 8);
}

function resolveAgentName(agentId) {
    const a = allAgents.find(a => a.id === agentId);
    return a ? a.name : agentId.slice(0, 8);
}

