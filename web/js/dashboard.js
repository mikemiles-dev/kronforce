// Kronforce - Dashboard home page (timeline chart, stats)
// --- Timeline Chart ---

function renderTimelineChart(containerId, data) {
    const container = document.getElementById(containerId);
    if (!data || data.length === 0) {
        container.innerHTML = '<div style="text-align:center;color:var(--text-muted);padding:20px;font-size:12px">No executions in this time window</div>';
        return;
    }

    const maxVal = Math.max(1, ...data.map(d => d.succeeded + d.failed + d.other));
    const chartHeight = 80;

    let html = '<div class="timeline-container">';
    html += '<div class="timeline-header">';
    html += '<div class="timeline-legend">';
    html += '<div class="timeline-legend-item"><div class="timeline-legend-dot" style="background:var(--success)"></div>Succeeded</div>';
    html += '<div class="timeline-legend-item"><div class="timeline-legend-dot" style="background:var(--danger)"></div>Failed</div>';
    html += '<div class="timeline-legend-item"><div class="timeline-legend-dot" style="background:var(--info)"></div>Other</div>';
    html += '</div></div>';
    html += '<div class="timeline-chart" style="height:' + chartHeight + 'px">';

    for (const d of data) {
        const total = d.succeeded + d.failed + d.other;
        const sH = Math.round((d.succeeded / maxVal) * chartHeight);
        const fH = Math.round((d.failed / maxVal) * chartHeight);
        const oH = Math.round((d.other / maxVal) * chartHeight);
        const time = d.time.slice(11); // HH:MM

        html += '<div class="timeline-bar-group" data-bucket="' + d.time + '" onmouseenter="showTimelineTooltip(event,\'' + d.time + '\',\'' + time + '\',' + d.succeeded + ',' + d.failed + ',' + d.other + ')" onmouseleave="hideTimelineTooltip()">';
        if (oH > 0) html += '<div class="timeline-bar other" style="height:' + oH + 'px"></div>';
        if (fH > 0) html += '<div class="timeline-bar failed" style="height:' + fH + 'px"></div>';
        if (sH > 0) html += '<div class="timeline-bar succeeded" style="height:' + sH + 'px"></div>';
        if (total === 0) html += '<div class="timeline-bar" style="height:1px;background:var(--border)"></div>';
        html += '</div>';
    }

    html += '</div>';

    // Time labels
    if (data.length > 0) {
        const first = data[0].time.slice(11);
        const last = data[data.length - 1].time.slice(11);
        const mid = data[Math.floor(data.length / 2)].time.slice(11);
        html += '<div class="timeline-labels"><span>' + first + '</span><span>' + mid + '</span><span>' + last + '</span></div>';
    }

    html += '</div>';
    container.innerHTML = html;
}

let tooltipEl = null;
let tooltipFetchController = null;

function showTimelineTooltip(event, bucket, time, succeeded, failed, other) {
    hideTimelineTooltip();

    const total = succeeded + failed + other;
    if (total === 0) return;

    const el = document.createElement('div');
    el.className = 'timeline-tooltip';

    let html = '<div class="tt-time">' + time + '</div>';
    html += '<div class="tt-summary">';
    html += '<span>\u2714 ' + succeeded + '</span>';
    html += '<span>\u2718 ' + failed + '</span>';
    if (other > 0) html += '<span>\u25CB ' + other + '</span>';
    html += '</div>';
    html += '<div class="tt-jobs"><span class="tt-loading">Loading jobs...</span></div>';
    el.innerHTML = html;

    document.body.appendChild(el);
    tooltipEl = el;

    // Position near the bar
    const rect = event.target.closest('.timeline-bar-group').getBoundingClientRect();
    el.style.left = Math.min(rect.left, window.innerWidth - 290) + 'px';
    el.style.top = (rect.top - el.offsetHeight - 8) + 'px';
    if (parseInt(el.style.top) < 0) {
        el.style.top = (rect.bottom + 8) + 'px';
    }

    // Fetch job detail
    api('GET', '/api/timeline-detail/' + encodeURIComponent(bucket))
        .then(data => {
            if (!tooltipEl) return;
            const jobsDiv = el.querySelector('.tt-jobs');
            if (!data || data.length === 0) {
                jobsDiv.innerHTML = '<span class="tt-loading">No job details</span>';
                return;
            }
            // Aggregate by job name
            const byJob = {};
            for (const d of data) {
                if (!byJob[d.job_name]) byJob[d.job_name] = { succeeded: 0, failed: 0, other: 0 };
                if (d.status === 'succeeded') byJob[d.job_name].succeeded += d.count;
                else if (d.status === 'failed' || d.status === 'timed_out') byJob[d.job_name].failed += d.count;
                else byJob[d.job_name].other += d.count;
            }
            let jobHtml = '';
            const sorted = Object.entries(byJob).sort((a, b) => {
                const ta = a[1].succeeded + a[1].failed + a[1].other;
                const tb = b[1].succeeded + b[1].failed + b[1].other;
                return tb - ta;
            }).slice(0, 5);
            for (const [name, counts] of sorted) {
                let statusParts = [];
                if (counts.succeeded > 0) statusParts.push('\u2714' + counts.succeeded);
                if (counts.failed > 0) statusParts.push('\u2718' + counts.failed);
                if (counts.other > 0) statusParts.push('\u25CB' + counts.other);
                jobHtml += '<div class="tt-job"><span class="tt-job-name">' + esc(name) + '</span><span>' + statusParts.join(' ') + '</span></div>';
            }
            if (Object.keys(byJob).length > 5) {
                jobHtml += '<div class="tt-job" style="color:var(--text-muted)">...and ' + (Object.keys(byJob).length - 5) + ' more</div>';
            }
            jobsDiv.innerHTML = jobHtml;
        })
        .catch(() => {
            if (tooltipEl) {
                const jobsDiv = el.querySelector('.tt-jobs');
                if (jobsDiv) jobsDiv.innerHTML = '';
            }
        });
}

function hideTimelineTooltip() {
    if (tooltipEl) {
        tooltipEl.remove();
        tooltipEl = null;
    }
}

async function fetchDashTimeline() {
    try {
        const data = await api('GET', '/api/timeline?minutes=15');
        renderTimelineChart('dash-timeline', data);
    } catch (e) {
        console.error('fetchDashTimeline:', e);
    }
}

async function fetchChartStats() {
    try {
        const data = await api('GET', '/api/stats/charts');
        renderDonutChart('dash-chart-outcomes', data.execution_outcomes);
        renderDonutChart('dash-chart-tasks', data.task_types);
        renderDonutChart('dash-chart-schedules', data.schedule_types);
    } catch (e) {
        console.error('fetchChartStats:', e);
        // Show empty state on failure
        ['dash-chart-outcomes', 'dash-chart-tasks', 'dash-chart-schedules'].forEach(id => {
            const el = document.getElementById(id);
            if (el) el.innerHTML = '<div class="donut-empty">No data</div>';
        });
    }
}

async function fetchJobTimeline(jobId) {
    try {
        const data = await api('GET', '/api/timeline/' + jobId + '?minutes=60');
        renderTimelineChart('job-timeline', data);
    } catch (e) {
        console.error('fetchJobTimeline:', e);
    }
}

// --- Dashboard Home ---

async function renderDashboard() {
    try {
        // Fetch all data in parallel
        const [jobsRes, eventsRes] = await Promise.all([
            api('GET', '/api/jobs?per_page=100'),
            api('GET', '/api/events?per_page=10'),
        ]);

        const jobs = jobsRes.data;
        const events = eventsRes.data;

        // Compute stats
        const totalJobs = jobs.length;
        const scheduled = jobs.filter(j => j.status === 'scheduled').length;
        const waiting = jobs.filter(j => j.depends_on.length > 0 && !j.deps_satisfied).length;
        const paused = jobs.filter(j => j.status === 'paused').length;

        let totalExecs = 0, totalSucceeded = 0, totalFailed = 0;
        for (const j of jobs) {
            totalExecs += j.execution_counts.total;
            totalSucceeded += j.execution_counts.succeeded;
            totalFailed += j.execution_counts.failed;
        }

        const onlineAgents = allAgents.filter(a => a.status === 'online').length;
        const totalAgents = allAgents.length;

        // Render stats cards
        document.getElementById('dash-stats').innerHTML =
            statCard(totalJobs, 'Total Jobs', 'neutral', "showPage('jobs')") +
            statCard(scheduled, 'Scheduled', 'info', "navJobsFiltered('scheduled')") +
            statCard(waiting, 'Waiting', 'warning', "navJobsFiltered('blocked')") +
            statCard(paused, 'Paused', 'neutral', "navJobsFiltered('paused')") +
            statCard(totalSucceeded, 'Succeeded', 'success', "navExecsFiltered('succeeded')") +
            statCard(totalFailed, 'Failed', 'danger', "navExecsFiltered('failed')") +
            statCard(onlineAgents + '/' + totalAgents, 'Agents Online', onlineAgents > 0 ? 'success' : 'neutral', "showPage('agents')") +
            statCard(totalExecs, 'Total Runs', 'neutral', "showPage('executions')");

        // Recent executions - collect from all jobs
        let recentExecs = [];
        for (const j of jobs) {
            if (j.last_execution) {
                recentExecs.push({ ...j.last_execution, job_name: j.name, job_id: j.id });
            }
        }
        recentExecs.sort((a, b) => {
            const ta = a.finished_at || '';
            const tb = b.finished_at || '';
            return tb.localeCompare(ta);
        });
        recentExecs = recentExecs.slice(0, 8);

        if (recentExecs.length > 0) {
            let html = '<table class="dash-mini-table"><tbody>';
            for (const e of recentExecs) {
                html += '<tr style="cursor:pointer" onclick="showJobDetail(\'' + e.job_id + '\')">';
                html += '<td><span class="job-name">' + esc(e.job_name) + '</span></td>';
                html += '<td>' + badge(e.status) + '</td>';
                html += '<td><span class="time-text">' + (e.finished_at ? fmtDate(e.finished_at) : 'running') + '</span></td>';
                html += '</tr>';
            }
            html += '</tbody></table>';
            document.getElementById('dash-recent-execs').innerHTML = html;
        } else {
            document.getElementById('dash-recent-execs').innerHTML = '<div class="empty-state" style="padding:20px"><p>No executions yet</p></div>';
        }

        // Recent events
        if (events.length > 0) {
            let html = '<div class="event-timeline">';
            for (const e of events.slice(0, 6)) {
                const icon = eventIcon(e.severity, e.kind);
                html += '<div class="event-item" style="padding:6px 10px">';
                html += '<div class="event-icon ' + e.severity + '" style="width:22px;height:22px;font-size:11px">' + icon + '</div>';
                html += '<div class="event-body"><div class="event-message" style="font-size:12px">' + esc(e.message) + '</div></div>';
                html += '<div class="event-time">' + fmtDate(e.timestamp) + '</div>';
                html += '</div>';
            }
            html += '</div>';
            document.getElementById('dash-recent-events').innerHTML = html;
        } else {
            document.getElementById('dash-recent-events').innerHTML = '<div class="empty-state" style="padding:20px"><p>No events yet</p></div>';
        }

        // Agents summary
        if (allAgents.length > 0) {
            let html = '<div style="display:flex;flex-direction:column;gap:8px;padding:4px 0">';
            for (const a of allAgents) {
                html += '<div style="display:flex;align-items:center;gap:8px;font-size:13px">';
                html += '<span class="agent-status-dot ' + a.status + '"></span>';
                html += '<span style="flex:1">' + esc(a.name) + '</span>';
                html += badge(a.status);
                html += '</div>';
            }
            html += '</div>';
            document.getElementById('dash-agents').innerHTML = html;
        } else {
            document.getElementById('dash-agents').innerHTML = '<div class="empty-state" style="padding:20px"><p>No agents</p></div>';
        }

        // Mini map
        renderDashMap(jobs);
        fetchDashTimeline();
        fetchChartStats();

    } catch (e) {
        console.error('renderDashboard:', e);
    }
}

function navJobsFiltered(filter) {
    jobSearch.statusFilter = filter;
    showPage('jobs');
    // Set the active filter button
    document.querySelectorAll('#status-filters .status-btn').forEach(b => {
        b.classList.toggle('active', b.dataset.status === filter || (!filter && b.dataset.status === ''));
    });
    fetchJobs(true);
}

function navExecsFiltered(status) {
    execSearch.statusFilter = status;
    showPage('executions');
    document.querySelectorAll('#exec-status-filters .status-btn').forEach(b => {
        b.classList.toggle('active', b.textContent.toLowerCase() === status || (!status && b.textContent === 'All'));
    });
    fetchAllExecutions();
}

function statCard(value, label, cls, onclick) {
    const click = onclick ? ' onclick="' + onclick + '" style="cursor:pointer"' : '';
    return '<div class="stat-card ' + cls + '"' + click + '><div class="stat-value">' + value + '</div><div class="stat-label">' + label + '</div></div>';
}

function renderDashMap(jobs) {
    // Reuse the full map renderer but target the dashboard SVG
    const svg = document.getElementById('dash-map-svg');
    if (!svg || jobs.length === 0) return;

    // Only show jobs with dependencies for a cleaner view
    const withDeps = new Set();
    for (const j of jobs) {
        if (j.depends_on.length > 0) {
            withDeps.add(j.id);
            for (const d of j.depends_on) withDeps.add(d.job_id);
        }
    }

    if (withDeps.size === 0) {
        svg.innerHTML = '';
        return;
    }

    const filtered = jobs.filter(j => withDeps.has(j.id));

    // Build layout (same logic as renderMap)
    const jobMap = {};
    for (const j of filtered) jobMap[j.id] = j;

    const children = {};
    const parents = {};
    for (const j of filtered) {
        children[j.id] = children[j.id] || [];
        parents[j.id] = parents[j.id] || [];
        for (const dep of j.depends_on) {
            if (withDeps.has(dep.job_id)) {
                children[dep.job_id] = children[dep.job_id] || [];
                children[dep.job_id].push(j.id);
                parents[j.id].push(dep.job_id);
            }
        }
    }

    const layers = {};
    const roots = filtered.filter(j => (parents[j.id] || []).length === 0).map(j => j.id);
    const visited = new Set();
    const queue = roots.map(id => ({ id, layer: 0 }));
    while (queue.length > 0) {
        const { id, layer } = queue.shift();
        if (visited.has(id)) { layers[id] = Math.max(layers[id] || 0, layer); continue; }
        visited.add(id);
        layers[id] = layer;
        for (const cid of (children[id] || [])) queue.push({ id: cid, layer: layer + 1 });
    }
    for (const j of filtered) { if (!visited.has(j.id)) layers[j.id] = 0; }

    const nodeW = 140, nodeH = 36, layerGap = 70, nodeGap = 12, padX = 20, padY = 16;
    const layerGroups = {};
    let maxLayer = 0;
    for (const [id, layer] of Object.entries(layers)) {
        layerGroups[layer] = layerGroups[layer] || [];
        layerGroups[layer].push(id);
        maxLayer = Math.max(maxLayer, layer);
    }

    const positions = {};
    let totalW = 0, totalH = 0;
    for (let l = 0; l <= maxLayer; l++) {
        const group = layerGroups[l] || [];
        const colX = padX + l * (nodeW + layerGap);
        for (let i = 0; i < group.length; i++) {
            const y = padY + i * (nodeH + nodeGap);
            positions[group[i]] = { x: colX, y };
            totalW = Math.max(totalW, colX + nodeW + padX);
            totalH = Math.max(totalH, y + nodeH + padY);
        }
    }

    svg.setAttribute('width', totalW);
    svg.setAttribute('height', totalH);
    svg.setAttribute('viewBox', '0 0 ' + totalW + ' ' + totalH);

    let svgHtml = '<defs><marker id="dash-arrow" viewBox="0 0 10 6" refX="10" refY="3" markerWidth="7" markerHeight="5" orient="auto-start-reverse"><path d="M 0 0 L 10 3 L 0 6 z" class="map-arrowhead"/></marker></defs>';

    for (const j of filtered) {
        for (const dep of j.depends_on) {
            const from = positions[dep.job_id];
            const to = positions[j.id];
            if (!from || !to) continue;
            const x1 = from.x + nodeW, y1 = from.y + nodeH / 2, x2 = to.x, y2 = to.y + nodeH / 2;
            svgHtml += '<path d="M ' + x1 + ' ' + y1 + ' C ' + (x1 + 30) + ' ' + y1 + ', ' + (x2 - 30) + ' ' + y2 + ', ' + x2 + ' ' + y2 + '" class="map-edge" stroke="var(--text-muted)" marker-end="url(#dash-arrow)"/>';
        }
    }

    for (const j of filtered) {
        const pos = positions[j.id];
        if (!pos) continue;
        let fill = 'var(--bg-tertiary)', stroke = 'var(--border)';
        const ls = j.last_execution ? j.last_execution.status : null;
        if (ls === 'succeeded') { fill = 'rgba(46,204,113,0.15)'; stroke = 'var(--success)'; }
        else if (ls === 'failed' || ls === 'timed_out') { fill = 'rgba(224,82,82,0.15)'; stroke = 'var(--danger)'; }
        else if (ls === 'running') { fill = 'rgba(62,139,255,0.15)'; stroke = 'var(--info)'; }

        svgHtml += '<g class="map-node" onclick="showJobDetail(\'' + j.id + '\')">';
        svgHtml += '<rect x="' + pos.x + '" y="' + pos.y + '" width="' + nodeW + '" height="' + nodeH + '" fill="' + fill + '" stroke="' + stroke + '" rx="5" ry="5"/>';
        svgHtml += '<text x="' + (pos.x + 8) + '" y="' + (pos.y + 14) + '" font-size="10" font-weight="600" fill="var(--text-primary)">' + esc(j.name) + '</text>';
        svgHtml += '<text x="' + (pos.x + 8) + '" y="' + (pos.y + 26) + '" font-size="9" fill="var(--text-muted)">' + j.status + (ls ? ' \u2022 ' + ls : '') + '</text>';
        svgHtml += '</g>';
    }

    svg.innerHTML = svgHtml;
}

