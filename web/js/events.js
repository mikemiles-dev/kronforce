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
        wrap.innerHTML = renderRichEmptyState({
            icon: '&#128276;',
            title: 'No events yet',
            description: 'Events are logged when jobs run, agents change status, API keys are managed, and output patterns match. Activity will appear here automatically.',
            actions: [
                { label: 'Create a Job', onclick: "showPage('jobs')", primary: true },
            ],
        });
        return;
    }

    let html = '<div class="event-timeline">';
    for (const e of events) {
        const icon = eventIcon(e.severity, e.kind);
        const jobLink = e.job_id ? '<span class="event-kind" style="cursor:pointer" onclick="showJobDetail(\'' + e.job_id + '\')">' + resolveJobName(e.job_id) + '</span>' : '';
        const agentLink = e.agent_id ? '<span class="event-kind">' + resolveAgentName(e.agent_id) + '</span>' : '';

        html += '<div class="event-item">';
        html += '<div class="event-icon ' + e.severity + '">' + icon + '</div>';
        html += '<div class="event-body">';
        html += '<div class="event-message">' + esc(e.message) + '</div>';
        html += '<div class="event-meta">';
        html += '<span class="event-kind">' + e.kind + '</span>';
        if (e.api_key_name) html += '<span class="event-kind" title="API Key: ' + (e.api_key_id || '') + '">&#128100; ' + esc(e.api_key_name) + '</span>';
        if (jobLink) html += jobLink;
        if (agentLink) html += agentLink;
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

