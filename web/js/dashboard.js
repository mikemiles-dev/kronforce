// Kronforce - Dashboard home page (timeline chart, stats, tabs)
// --- Dashboard Tabs ---
let currentDashTab = 'overview';

function showDashTab(tabName) {
    if (tabName === 'charts') tabName = 'overview'; // charts merged into overview
    currentDashTab = tabName;
    document.querySelectorAll('.dash-tab-panel').forEach(p => {
        p.classList.toggle('active', p.dataset.tab === tabName);
    });
    document.querySelectorAll('.dash-tab-btn').forEach(b => {
        b.classList.toggle('active', b.textContent.toLowerCase() === tabName);
    });
}

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
    // Clean up any orphaned tooltips
    document.querySelectorAll('.timeline-tooltip').forEach(el => el.remove());

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

function renderDashGroupSummary(jobs) {
    const el = document.getElementById('dash-top-groups');
    if (!el) return;

    const counts = {};
    for (const j of jobs) {
        if (j.group) {
            counts[j.group] = (counts[j.group] || 0) + 1;
        }
    }

    const sorted = Object.entries(counts).sort((a, b) => b[1] - a[1]).slice(0, 5);
    if (sorted.length === 0) {
        el.innerHTML = '<div style="padding:12px;color:var(--text-muted);font-size:13px">No groups configured</div>';
        return;
    }

    let html = '<div style="display:flex;flex-direction:column;gap:6px;padding:4px 0">';
    for (const [name, count] of sorted) {
        const color = groupColor(name);
        html += '<div style="display:flex;align-items:center;gap:8px;font-size:13px;cursor:pointer;padding:4px 8px;border-radius:4px" onclick="groupFilter=\'' + esc(name) + '\';showPage(\'jobs\')">';
        html += '<span style="width:8px;height:8px;border-radius:50%;background:' + color + ';flex-shrink:0"></span>';
        html += '<span style="flex:1">' + esc(name) + '</span>';
        html += '<span style="color:var(--text-secondary);font-size:12px">' + count + ' job' + (count !== 1 ? 's' : '') + '</span>';
        html += '</div>';
    }
    html += '</div>';
    el.innerHTML = html;
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
        const waiting = jobs.filter(j => (j.depends_on || []).length > 0 && !j.deps_satisfied).length;
        const paused = jobs.filter(j => j.status === 'paused').length;

        let totalExecs = 0, totalSucceeded = 0, totalFailed = 0;
        for (const j of jobs) {
            const c = j.execution_counts || {};
            totalExecs += c.total || 0;
            totalSucceeded += c.succeeded || 0;
            totalFailed += c.failed || 0;
        }

        const onlineAgents = allAgents.filter(a => a.status === 'online').length;
        const totalAgents = allAgents.length;
        const groupSet = new Set(jobs.map(j => j.group || 'Default'));
        const totalGroups = groupSet.size;

        const running = jobs.filter(j => j.last_execution && j.last_execution.status === 'running').length;

        // Render stats cards
        document.getElementById('dash-stats').innerHTML =
            statCard(totalJobs, 'Total Jobs', 'neutral', "showPage('jobs')") +
            (running > 0 ? statCard(running, 'Running', 'info', "navJobsFiltered('running')") : '') +
            statCard(scheduled, 'Scheduled', 'info', "navJobsFiltered('scheduled')") +
            statCard(totalFailed, 'Failed', 'danger', "navExecsFiltered('failed')") +
            statCard(waiting, 'Waiting', 'warning', "navJobsFiltered('blocked')") +
            statCard(paused, 'Paused', 'neutral', "navJobsFiltered('paused')") +
            statCard(totalSucceeded, 'Succeeded', 'success', "navExecsFiltered('succeeded')") +
            statCard(onlineAgents + '/' + totalAgents, 'Agents Online', onlineAgents > 0 ? 'success' : 'neutral', "showPage('agents')") +
            statCard(totalGroups, 'Groups', 'neutral', "showPage('jobs')") +
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
            // Track which job we've seen first (latest per job)
            const seenJobs = new Set();
            let html = '<table class="dash-mini-table"><tbody>';
            for (const e of recentExecs) {
                const isLatest = !seenJobs.has(e.job_id);
                seenJobs.add(e.job_id);
                html += '<tr style="cursor:pointer' + (isLatest ? ';border-left:3px solid var(--accent)' : '') + '" onclick="showJobDetail(\'' + e.job_id + '\')">';
                html += '<td><span class="job-name">' + esc(e.job_name) + '</span>' + groupBadge((jobs || []).find(j => j.id === e.job_id)?.group) + '</td>';
                html += '<td>' + badge(e.status) + '</td>';
                html += '<td><span class="time-text">' + (e.finished_at ? fmtDate(e.finished_at) : '<span class="badge badge-running">running</span>') + '</span></td>';
                html += '</tr>';
            }
            html += '</tbody></table>';
            document.getElementById('dash-recent-execs').innerHTML = html;
        } else {
            document.getElementById('dash-recent-execs').innerHTML = '<div class="empty-state" style="padding:20px"><p>No executions yet</p></div>';
        }

        // Currently running jobs
        const runningJobs = jobs.filter(j => j.last_execution && j.last_execution.status === 'running');
        const runningSection = document.getElementById('dash-running-section');
        if (runningSection) {
            if (runningJobs.length > 0) {
                let rhtml = '<div class="card" style="margin-bottom:16px;border-left:3px solid var(--accent)">';
                rhtml += '<div class="card-header" style="cursor:pointer" onclick="navJobsFiltered(\'running\')"><h3>&#9654; Running Now (' + runningJobs.length + ') <span style="font-size:11px;color:var(--accent)">&rarr;</span></h3></div>';
                rhtml += '<div style="display:flex;gap:8px;flex-wrap:wrap;padding:0 16px 12px">';
                for (const j of runningJobs) {
                    rhtml += '<div style="padding:6px 12px;background:rgba(62,139,255,0.1);border:1px solid var(--accent);border-radius:6px;cursor:pointer;font-size:12px" onclick="showJobDetail(\'' + j.id + '\')">';
                    rhtml += '<span style="font-weight:600">' + esc(j.name) + '</span>';
                    if (j.last_execution.started_at) rhtml += ' <span style="color:var(--text-muted);font-size:11px">since ' + fmtDate(j.last_execution.started_at) + '</span>';
                    rhtml += '</div>';
                }
                rhtml += '</div></div>';
                runningSection.innerHTML = rhtml;
            } else {
                runningSection.innerHTML = '';
            }
        }

        // Recently failed jobs
        const failedJobs = jobs.filter(j => j.last_execution && (j.last_execution.status === 'failed' || j.last_execution.status === 'timed_out'));
        const failedSection = document.getElementById('dash-failed-section');
        if (failedSection) {
            if (failedJobs.length > 0) {
                let fhtml = '<div class="card" style="margin-bottom:16px;border-left:3px solid var(--danger)">';
                fhtml += '<div class="card-header" style="cursor:pointer" onclick="navExecsFiltered(\'failed\')"><h3>&#10060; Recently Failed (' + failedJobs.length + ') <span style="font-size:11px;color:var(--accent)">&rarr;</span></h3></div>';
                fhtml += '<table class="dash-mini-table"><tbody>';
                for (const j of failedJobs.slice(0, 8)) {
                    fhtml += '<tr style="cursor:pointer" onclick="showJobDetail(\'' + j.id + '\')">';
                    fhtml += '<td><span class="job-name">' + esc(j.name) + '</span>' + groupBadge(j.group) + '</td>';
                    fhtml += '<td>' + badge(j.last_execution.status) + '</td>';
                    fhtml += '<td><span class="time-text">' + (j.last_execution.finished_at ? fmtDate(j.last_execution.finished_at) : '') + '</span></td>';
                    fhtml += '</tr>';
                }
                fhtml += '</tbody></table></div>';
                failedSection.innerHTML = fhtml;
            } else {
                failedSection.innerHTML = '';
            }
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

        // Infrastructure: stages + Cytoscape map
        renderDashStages(jobs);
        renderDashCytoMap(jobs);
        renderDashGroupSummary(jobs);
        fetchDashTimeline();
        fetchChartStats();

        // Restore active tab
        showDashTab(currentDashTab);

    } catch (e) {
        console.error('renderDashboard:', e);
    }
}

function navJobsFiltered(filter) {
    jobsTab = 'list';
    jobSearch.statusFilter = filter;
    showPage('jobs');
    // Sync filter button state after page renders
    setTimeout(function() {
        document.querySelectorAll('#status-filters .status-btn').forEach(b => {
            b.classList.toggle('active', b.dataset.status === filter || (!filter && b.dataset.status === ''));
        });
    }, 50);
}

function navExecsFiltered(status) {
    execSearch.statusFilter = status;
    showPage('executions');
    // Sync button state — match by the onclick attribute containing the status value
    setTimeout(function() {
        document.querySelectorAll('#exec-status-filters .status-btn').forEach(b => {
            const onclick = b.getAttribute('onclick') || '';
            const isMatch = status ? onclick.includes("'" + status + "'") : onclick.includes("''");
            b.classList.toggle('active', isMatch);
        });
    }, 50);
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

function renderDashStages(jobs) {
    const el = document.getElementById('dash-stages');
    if (!el) return;
    if (jobs.length === 0) {
        el.innerHTML = '<div class="empty-state" style="padding:20px"><p>No jobs</p></div>';
        return;
    }
    // Group jobs
    const jobsByGroup = {};
    for (const j of jobs) {
        const g = j.group || 'Default';
        if (!jobsByGroup[g]) jobsByGroup[g] = [];
        jobsByGroup[g].push(j);
    }
    const sortedGroups = Object.keys(jobsByGroup).sort((a, b) => a === 'Default' ? -1 : b === 'Default' ? 1 : a.localeCompare(b));

    // Reuse renderPipelineView via the ID swap trick
    el.id = 'groups-grid';
    if (typeof renderPipelineView === 'function') {
        renderPipelineView(sortedGroups.filter(g => jobsByGroup[g].length > 0), jobsByGroup);
    }
    el.id = 'dash-stages';
}

let dashCyInstance = null;

function renderDashCytoMap(jobs) {
    const container = document.getElementById('dash-map-container');
    if (!container) return;
    if (typeof cytoscape === 'undefined') {
        container.innerHTML = '<div class="empty-state" style="padding:20px"><p>Map library not loaded</p></div>';
        return;
    }
    if (jobs.length === 0) {
        if (dashCyInstance) { dashCyInstance.destroy(); dashCyInstance = null; }
        container.innerHTML = '<div class="empty-state" style="padding:20px"><p>No jobs</p></div>';
        return;
    }

    // Build nodes and edges
    const elements = [];
    for (const j of jobs) {
        const ls = j.last_execution ? j.last_execution.status : 'idle';
        let color = '#555';
        if (ls === 'succeeded') color = '#2ecc71';
        else if (ls === 'failed' || ls === 'timed_out') color = '#e05252';
        else if (ls === 'running') color = '#3e8bff';
        elements.push({ data: { id: j.id, label: j.name, color: color, group: j.group || 'Default' } });
        for (const d of (j.depends_on || [])) {
            elements.push({ data: { source: d.job_id, target: j.id } });
        }
    }

    // Event-based edges (dashed)
    if (jobs.some(j => j.schedule && j.schedule.type === 'event')) {
        const nameToId = {};
        for (const j of jobs) nameToId[j.name] = j.id;
        for (const j of jobs) {
            if (j.schedule && j.schedule.type === 'event' && j.schedule.value && j.schedule.value.job_name_filter) {
                const srcId = nameToId[j.schedule.value.job_name_filter];
                if (srcId) elements.push({ data: { source: srcId, target: j.id, dashed: true } });
            }
        }
    }

    if (dashCyInstance) dashCyInstance.destroy();

    const isDark = document.documentElement.getAttribute('data-theme') !== 'light';
    const textColor = isDark ? '#e0e2eb' : '#1a1d2e';

    dashCyInstance = cytoscape({
        container: container,
        elements: elements,
        style: [
            { selector: 'node', style: {
                'label': 'data(label)', 'background-color': 'data(color)',
                'color': textColor, 'font-size': '11px', 'text-valign': 'bottom',
                'text-margin-y': 6, 'width': 28, 'height': 28, 'border-width': 2,
                'border-color': 'data(color)'
            }},
            { selector: 'edge', style: {
                'width': 2, 'line-color': '#3e8bff', 'target-arrow-color': '#3e8bff',
                'target-arrow-shape': 'triangle', 'curve-style': 'bezier', 'opacity': 0.6
            }},
            { selector: 'edge[dashed]', style: { 'line-style': 'dashed', 'line-color': '#e6a817', 'target-arrow-color': '#e6a817' } }
        ],
        layout: { name: 'breadthfirst', directed: true, padding: 30, spacingFactor: 1.2 },
        userZoomingEnabled: true, userPanningEnabled: true, boxSelectionEnabled: false
    });

    dashCyInstance.on('tap', 'node', function(evt) {
        showJobDetail(evt.target.id());
    });

    // Show map controls
    var ctrl = document.getElementById('dash-map-controls');
    if (ctrl) ctrl.style.display = '';

    // Sync zoom slider
    dashCyInstance.on('zoom', function() {
        var slider = document.getElementById('dash-map-zoom-slider');
        if (slider) slider.value = Math.round(dashCyInstance.zoom() * 100);
    });

    setTimeout(function() { if (dashCyInstance) dashCyInstance.fit(undefined, 30); }, 100);
}

