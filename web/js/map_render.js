// Kronforce - Dependency map (Cytoscape) and mini-map rendering

var cyInstance = null;

var _recentStatuses = {};

async function renderMap() {
    var mapEl = document.getElementById('map-container');
    if (mapEl) mapEl.innerHTML = '<div class="loading-bar"></div><div class="loading-placeholder">Loading map...</div>';
    let jobs;
    try {
        var [res, statuses] = await Promise.all([
            api('GET', '/api/jobs?per_page=1000'),
            api('GET', '/api/executions/recent-statuses').catch(function() { return {}; }),
        ]);
        jobs = typeof applyJobFilters === 'function' ? applyJobFilters(res.data) : res.data;
        _recentStatuses = statuses || {};
    } catch (e) {
        console.error('renderMap:', e);
        return;
    }

    const container = document.getElementById('map-container');

    if (jobs.length === 0) {
        if (cyInstance) { cyInstance.destroy(); cyInstance = null; }
        container.innerHTML = renderRichEmptyState({
            icon: '&#9741;',
            title: 'No jobs to display',
            description: 'The dependency map visualizes how jobs depend on each other. Create jobs with dependencies to see the graph.',
            actions: [{ label: 'Create a Job', onclick: 'openCreateModal()', primary: true }],
        });
        return;
    }

    // Build Cytoscape elements
    const jobMap = {};
    for (const j of jobs) jobMap[j.id] = j;

    const elements = [];
    const edgeSet = new Set();

    // Nodes
    for (const j of jobs) {
        const lastStatus = j.last_execution ? j.last_execution.status : 'none';
        elements.push({
            data: {
                id: j.id,
                label: j.name,
                group: j.group || 'Default',
                status: j.status,
                lastStatus: lastStatus,
                taskType: j.task ? j.task.type : 'unknown',
                schedType: j.schedule ? j.schedule.type : 'unknown',
                approval: j.approval_required || false,
                priority: j.priority || 0,
            }
        });
    }

    // Dependency edges
    for (const j of jobs) {
        for (const dep of j.depends_on) {
            if (!jobMap[dep.job_id]) continue;
            const eid = dep.job_id + '->' + j.id;
            if (!edgeSet.has(eid)) {
                edgeSet.add(eid);
                const label = dep.within_secs ? 'within ' + fmtSeconds(dep.within_secs) : '';
                elements.push({ data: { id: eid, source: dep.job_id, target: j.id, label: label, edgeType: 'dependency' } });
            }
        }
    }

    // Event trigger edges
    for (const j of jobs) {
        if (j.schedule.type !== 'event' || !j.schedule.value || !j.schedule.value.job_name_filter) continue;
        const filter = j.schedule.value.job_name_filter.toLowerCase();
        const kind = j.schedule.value.kind_pattern || '*';
        for (const src of jobs) {
            if (src.id === j.id) continue;
            if (!src.name.toLowerCase().includes(filter)) continue;
            const eid = src.id + '=>' + j.id;
            if (!edgeSet.has(eid)) {
                edgeSet.add(eid);
                elements.push({ data: { id: eid, source: src.id, target: j.id, label: kind, edgeType: 'event' } });
            }
        }
    }

    // Colors
    function statusColor(s) {
        if (s === 'succeeded') return '#2ecc71';
        if (s === 'failed' || s === 'timed_out') return '#e05252';
        if (s === 'running') return '#3e8bff';
        if (s === 'pending_approval') return '#e6a817';
        return '#7c8298';
    }

    // Group → distinct pastel background
    const groupPalette = {
        light: ['#dbeafe','#d1fae5','#fef3c7','#fce7f3','#ede9fe','#ffedd5','#ccfbf1','#e0e7ff'],
        dark:  ['#1e3a5f','#1a3d2e','#3d3520','#3d1f2e','#2d2650','#3d2a1a','#1a3d38','#252a50'],
    };
    function groupBg(group, dark) {
        let h = 0;
        for (let i = 0; i < group.length; i++) h = ((h << 5) - h + group.charCodeAt(i)) | 0;
        const pal = dark ? groupPalette.dark : groupPalette.light;
        return pal[Math.abs(h) % pal.length];
    }

    // Build execution history sparkline SVG for a node
    function historySparkline(jobId) {
        var runs = _recentStatuses[jobId];
        if (!runs || runs.length === 0) return null;
        // Show last 10, oldest first (API returns newest first)
        var recent = runs.slice(0, 10).reverse();
        var w = recent.length * 10;
        var svg = '<svg xmlns="http://www.w3.org/2000/svg" width="' + w + '" height="8" viewBox="0 0 ' + w + ' 8">';
        for (var i = 0; i < recent.length; i++) {
            var s = recent[i].status;
            var c = '#7c8298';
            if (s === 'succeeded') c = '#2ecc71';
            else if (s === 'failed' || s === 'timed_out') c = '#e05252';
            else if (s === 'running') c = '#3e8bff';
            else if (s === 'cancelled' || s === 'skipped') c = '#a0a8c0';
            svg += '<rect x="' + (i * 10 + 1) + '" y="1" width="7" height="6" rx="1.5" fill="' + c + '"/>';
        }
        svg += '</svg>';
        return 'data:image/svg+xml,' + encodeURIComponent(svg);
    }

    // Task type SVG icons (16x16, white fill for contrast)
    function taskIcon(type) {
        const icons = {
            shell: '<path d="M4 3l5 5-5 5" stroke="%23666" fill="none" stroke-width="1.5" stroke-linecap="round"/><line x1="10" y1="13" x2="14" y2="13" stroke="%23666" stroke-width="1.5" stroke-linecap="round"/>',
            http: '<circle cx="8" cy="8" r="6" stroke="%23666" fill="none" stroke-width="1.2"/><path d="M2 8h12M8 2c-2 2-2 10 0 12M8 2c2 2 2 10 0 12" stroke="%23666" fill="none" stroke-width="1"/>',
            sql: '<ellipse cx="8" cy="5" rx="5" ry="2.5" stroke="%23666" fill="none" stroke-width="1.2"/><path d="M3 5v6c0 1.4 2.2 2.5 5 2.5s5-1.1 5-2.5V5" stroke="%23666" fill="none" stroke-width="1.2"/>',
            mcp: '<rect x="3" y="3" width="10" height="10" rx="2" stroke="%23666" fill="none" stroke-width="1.2"/><circle cx="8" cy="8" r="2" fill="%23666"/>',
            custom: '<path d="M8 2L14 6v4l-6 4-6-4V6z" stroke="%23666" fill="none" stroke-width="1.2"/>',
        };
        const path = icons[type] || icons.shell;
        return 'data:image/svg+xml,' + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16">' + path + '</svg>');
    }

    // Theme
    const isDark = document.documentElement.getAttribute('data-theme') !== 'light';
    const textColor = isDark ? '#e0e2eb' : '#1a1d2e';
    const subtextColor = isDark ? '#8890a8' : '#6b7280';

    // Destroy previous instance
    if (cyInstance) { cyInstance.destroy(); cyInstance = null; }
    container.innerHTML = '';

    // Check for saved positions
    const savedPositions = localStorage.getItem('kf-mapPositions');
    let hasSaved = false;
    if (savedPositions) {
        try {
            const sp = JSON.parse(savedPositions);
            hasSaved = elements.some(e => e.data && !e.data.source && sp[e.data.id]);
        } catch(e) {}
    }

    cyInstance = cytoscape({
        container: container,
        elements: elements,
        style: [
            {
                selector: 'node',
                style: {
                    'label': 'data(label)',
                    'text-valign': 'top',
                    'text-halign': 'center',
                    'text-margin-y': 6,
                    'font-size': 13,
                    'font-family': '-apple-system, BlinkMacSystemFont, Segoe UI, Roboto, sans-serif',
                    'color': textColor,
                    'background-color': function(ele) { return statusColor(ele.data('lastStatus')); },
                    'background-opacity': 0.15,
                    'border-width': 3,
                    'border-color': function(ele) { return statusColor(ele.data('lastStatus')); },
                    'shape': 'round-rectangle',
                    'width': function(ele) { return Math.max(120, ele.data('label').length * 9 + 30); },
                    'height': function(ele) { return _recentStatuses[ele.id()] ? 52 : 40; },
                    'text-wrap': 'ellipsis',
                    'text-max-width': function(ele) { return Math.max(100, ele.data('label').length * 9 + 10); },
                    'background-image': function(ele) { return historySparkline(ele.id()) || 'none'; },
                    'background-image-opacity': 1,
                    'background-width': function(ele) {
                        var runs = _recentStatuses[ele.id()];
                        return runs ? Math.min(runs.length * 10, 100) + 'px' : '0px';
                    },
                    'background-height': '8px',
                    'background-position-y': '80%',
                    'background-clip': 'none',
                    'background-image-containment': 'over',
                }
            },
            {
                selector: 'node[?approval]',
                style: {
                    'border-style': 'dashed',
                    'border-width': 3.5,
                }
            },
            {
                selector: 'node[lastStatus = "running"]',
                style: {
                    'underlay-opacity': 0.35,
                    'underlay-padding': 9,
                    'border-width': 4,
                }
            },
            {
                selector: 'node[lastStatus = "failed"], node[lastStatus = "timed_out"]',
                style: {
                    'underlay-opacity': 0.25,
                    'underlay-padding': 7,
                }
            },
            {
                selector: 'node:active',
                style: {
                    'overlay-opacity': 0.06,
                }
            },
            {
                selector: 'node:selected',
                style: {
                    'border-color': '#3e8bff',
                    'border-width': 4,
                    'underlay-color': '#3e8bff',
                    'underlay-opacity': 0.2,
                }
            },
            {
                selector: 'edge[edgeType="dependency"]',
                style: {
                    'width': 2.5,
                    'line-color': isDark ? '#4a5070' : '#a0a8c0',
                    'target-arrow-color': isDark ? '#6a7090' : '#8890a8',
                    'target-arrow-shape': 'triangle',
                    'arrow-scale': 1.3,
                    'curve-style': 'bezier',
                    'label': 'data(label)',
                    'font-size': '10px',
                    'color': subtextColor,
                    'text-rotation': 'autorotate',
                    'text-margin-y': -10,
                    'text-background-color': isDark ? '#1a1b2e' : '#ffffff',
                    'text-background-opacity': 0.9,
                    'text-background-padding': '3px',
                    'text-background-shape': 'round-rectangle',
                }
            },
            {
                selector: 'edge[edgeType="event"]',
                style: {
                    'width': 2.5,
                    'line-color': '#e6a817',
                    'line-style': 'dashed',
                    'line-dash-pattern': [10, 5],
                    'target-arrow-color': '#e6a817',
                    'target-arrow-shape': 'triangle',
                    'arrow-scale': 1.3,
                    'curve-style': 'bezier',
                    'label': function(ele) { return '\u26A1 ' + ele.data('label'); },
                    'font-size': '10px',
                    'color': '#d49b10',
                    'text-rotation': 'autorotate',
                    'text-margin-y': -10,
                    'text-background-color': isDark ? '#1a1b2e' : '#ffffff',
                    'text-background-opacity': 0.9,
                    'text-background-padding': '3px',
                    'text-background-shape': 'round-rectangle',
                }
            },
        ],
        layout: { name: 'preset' },
        minZoom: 0.15,
        maxZoom: 3,
        wheelSensitivity: 0.3,
    });

    // Click node → job detail
    cyInstance.on('tap', 'node', function(evt) {
        showJobDetail(evt.target.id());
    });

    // Hover tooltip with execution history
    var mapTooltip = document.getElementById('map-tooltip');
    if (!mapTooltip) {
        mapTooltip = document.createElement('div');
        mapTooltip.id = 'map-tooltip';
        mapTooltip.style.cssText = 'position:fixed;z-index:9999;pointer-events:none;display:none;background:var(--bg-secondary);border:1px solid var(--border);border-radius:8px;padding:10px 12px;font-size:12px;box-shadow:0 4px 12px rgba(0,0,0,0.2);max-width:280px';
        document.body.appendChild(mapTooltip);
    }

    cyInstance.on('mouseover', 'node', function(evt) {
        var node = evt.target;
        var id = node.id();
        var runs = _recentStatuses[id] || [];
        var data = node.data();
        var html = '<div style="font-weight:600;margin-bottom:4px">' + esc(data.label) + '</div>';
        html += '<div style="color:var(--text-muted);margin-bottom:6px">' + esc(data.group || 'Default') + ' &middot; ' + esc(data.taskType) + '</div>';
        if (runs.length > 0) {
            html += '<div style="margin-bottom:4px;font-size:11px;color:var(--text-secondary)">Last ' + runs.length + ' runs:</div>';
            html += '<div style="display:flex;gap:3px;flex-wrap:wrap">';
            var recent = runs.slice(0, 10).reverse();
            for (var i = 0; i < recent.length; i++) {
                var s = recent[i].status;
                var c = '#7c8298';
                if (s === 'succeeded') c = '#2ecc71';
                else if (s === 'failed' || s === 'timed_out') c = '#e05252';
                else if (s === 'running') c = '#3e8bff';
                else if (s === 'cancelled' || s === 'skipped') c = '#a0a8c0';
                var ago = typeof fmtDateRelative === 'function' ? fmtDateRelative(recent[i].started_at) : fmtDate(recent[i].started_at);
                html += '<div title="' + esc(s) + ' — ' + esc(ago) + '" style="width:18px;height:14px;border-radius:3px;background:' + c + ';opacity:0.9"></div>';
            }
            html += '</div>';
        } else {
            html += '<div style="color:var(--text-muted);font-size:11px">No executions yet</div>';
        }
        mapTooltip.innerHTML = html;
        mapTooltip.style.display = '';
        var r = evt.renderedPosition || evt.position;
        var containerRect = container.getBoundingClientRect();
        mapTooltip.style.left = (containerRect.left + r.x + 15) + 'px';
        mapTooltip.style.top = (containerRect.top + r.y - 10) + 'px';
    });

    cyInstance.on('mouseout', 'node', function() {
        mapTooltip.style.display = 'none';
    });

    cyInstance.on('viewport', function() {
        mapTooltip.style.display = 'none';
    });

    // Save positions when nodes are dragged
    cyInstance.on('dragfree', 'node', function() {
        saveMapPositions();
    });

    function fitAndSync() {
        cyInstance.fit(undefined, 30);
        const slider = document.getElementById('map-zoom-slider');
        if (slider) slider.value = Math.round(cyInstance.zoom() * 100);
    }

    // Restore saved positions or run auto-layout
    if (hasSaved) {
        try {
            const positions = JSON.parse(savedPositions);
            cyInstance.nodes().forEach(function(n) {
                if (positions[n.id()]) n.position(positions[n.id()]);
            });
        } catch (e) {}
        fitAndSync();
    } else {
        cyInstance.layout({
            name: 'breadthfirst',
            directed: true,
            spacingFactor: 1.2,
            avoidOverlap: true,
            padding: 30,
            stop: fitAndSync,
        }).run();
    }

    // Sync zoom slider
    const slider = document.getElementById('map-zoom-slider');
    if (slider) {
        cyInstance.on('zoom', function() {
            slider.value = Math.round(cyInstance.zoom() * 100);
        });
    }

    // Show controls
    const controls = document.getElementById('map-controls');
    if (controls) controls.style.display = '';
}

function saveMapPositions() {
    if (!cyInstance) return;
    const positions = {};
    cyInstance.nodes().forEach(function(n) {
        positions[n.id()] = n.position();
    });
    localStorage.setItem('kf-mapPositions', JSON.stringify(positions));
}

function clearMapPositions() {
    localStorage.removeItem('kf-mapPositions');
    renderMap();
}

// --- Mini Dependency Map ---

var miniCyInstance = null;

function renderMiniMap(job) {
    const card = document.getElementById('mini-map-card');
    const container = document.getElementById('mini-map-svg');

    // Collect related jobs
    const relatedIds = new Set();
    relatedIds.add(job.id);
    for (const dep of job.depends_on) relatedIds.add(dep.job_id);
    for (const j of allJobs) {
        for (const dep of j.depends_on) {
            if (dep.job_id === job.id) relatedIds.add(j.id);
        }
        // Event trigger connections
        if (j.schedule && j.schedule.type === 'event' && j.schedule.value && j.schedule.value.job_name_filter) {
            const filter = j.schedule.value.job_name_filter.toLowerCase();
            if (job.name.toLowerCase().includes(filter)) relatedIds.add(j.id);
            if (relatedIds.has(j.id) && allJobs.some(s => s.name.toLowerCase().includes(filter) && s.id !== j.id)) {
                allJobs.filter(s => s.name.toLowerCase().includes(filter) && s.id !== j.id).forEach(s => relatedIds.add(s.id));
            }
        }
        if (job.schedule && job.schedule.type === 'event' && job.schedule.value && job.schedule.value.job_name_filter) {
            const filter = job.schedule.value.job_name_filter.toLowerCase();
            if (j.name.toLowerCase().includes(filter)) relatedIds.add(j.id);
        }
    }

    if (relatedIds.size <= 1) {
        container.innerHTML = '<div style="padding:40px;text-align:center;color:var(--text-muted)">No dependencies or event connections</div>';
        return;
    }

    const jobMap = {};
    for (const j of allJobs) jobMap[j.id] = j;
    jobMap[job.id] = job;

    const isDark = document.documentElement.getAttribute('data-theme') !== 'light';
    const textColor = isDark ? '#e0e2eb' : '#1a1d2e';

    function statusColor(s) {
        if (s === 'succeeded') return '#2ecc71';
        if (s === 'failed' || s === 'timed_out') return '#e05252';
        if (s === 'running') return '#3e8bff';
        return '#7c8298';
    }

    // Build elements
    const elements = [];
    const edgeSet = new Set();

    for (const id of relatedIds) {
        const j = jobMap[id];
        if (!j) continue;
        const lastStatus = j.last_execution ? j.last_execution.status : 'none';
        elements.push({
            data: {
                id: j.id,
                label: j.name,
                lastStatus: lastStatus,
                isCurrent: j.id === job.id,
            }
        });
    }

    // Dependency edges
    for (const id of relatedIds) {
        const j = jobMap[id];
        if (!j) continue;
        for (const dep of j.depends_on) {
            if (!relatedIds.has(dep.job_id)) continue;
            const eid = dep.job_id + '->' + j.id;
            if (!edgeSet.has(eid)) {
                edgeSet.add(eid);
                const label = dep.within_secs ? fmtSeconds(dep.within_secs) : '';
                elements.push({ data: { id: eid, source: dep.job_id, target: j.id, label: label, edgeType: 'dep' } });
            }
        }
    }

    // Event trigger edges
    for (const id of relatedIds) {
        const j = jobMap[id];
        if (!j || j.schedule.type !== 'event' || !j.schedule.value || !j.schedule.value.job_name_filter) continue;
        const filter = j.schedule.value.job_name_filter.toLowerCase();
        for (const sid of relatedIds) {
            if (sid === id) continue;
            const s = jobMap[sid];
            if (s && s.name.toLowerCase().includes(filter)) {
                const eid = sid + '=>' + id;
                if (!edgeSet.has(eid)) {
                    edgeSet.add(eid);
                    elements.push({ data: { id: eid, source: sid, target: id, label: j.schedule.value.kind_pattern || '*', edgeType: 'event' } });
                }
            }
        }
    }

    // Render
    if (miniCyInstance) { miniCyInstance.destroy(); miniCyInstance = null; }
    container.innerHTML = '';
    container.style.height = '200px';
    container.style.width = '100%';

    miniCyInstance = cytoscape({
        container: container,
        elements: elements,
        style: [
            {
                selector: 'node',
                style: {
                    'label': 'data(label)',
                    'text-valign': 'center',
                    'text-halign': 'center',
                    'font-size': 10,
                    'font-family': '-apple-system, BlinkMacSystemFont, Segoe UI, Roboto, sans-serif',
                    'color': textColor,
                    'background-color': function(ele) {
                        const s = ele.data('lastStatus');
                        if (s === 'succeeded') return isDark ? 'rgba(46,204,113,0.15)' : 'rgba(46,204,113,0.1)';
                        if (s === 'failed' || s === 'timed_out') return isDark ? 'rgba(224,82,82,0.15)' : 'rgba(224,82,82,0.1)';
                        return isDark ? '#252840' : '#f0f1f5';
                    },
                    'border-width': function(ele) { return ele.data('isCurrent') ? 3 : 2; },
                    'border-color': function(ele) {
                        if (ele.data('isCurrent')) return '#3e8bff';
                        return statusColor(ele.data('lastStatus'));
                    },
                    'shape': 'round-rectangle',
                    'width': function(ele) { return Math.max(100, ele.data('label').length * 8 + 20); },
                    'height': 32,
                    'text-wrap': 'ellipsis',
                    'text-max-width': function(ele) { return Math.max(80, ele.data('label').length * 8); },
                }
            },
            {
                selector: 'node[?isCurrent]',
                style: { 'underlay-color': '#3e8bff', 'underlay-opacity': 0.15, 'underlay-padding': 4, 'underlay-shape': 'round-rectangle' }
            },
            {
                selector: 'edge[edgeType="dep"]',
                style: {
                    'width': 2, 'line-color': isDark ? '#4a5070' : '#a0a8c0',
                    'target-arrow-color': isDark ? '#6a7090' : '#8890a8',
                    'target-arrow-shape': 'triangle', 'curve-style': 'bezier',
                    'label': 'data(label)', 'font-size': 8, 'color': '#7c8298',
                    'text-rotation': 'autorotate', 'text-margin-y': -8,
                    'text-background-color': isDark ? '#1a1b2e' : '#fff',
                    'text-background-opacity': 0.9, 'text-background-padding': '2px',
                }
            },
            {
                selector: 'edge[edgeType="event"]',
                style: {
                    'width': 2, 'line-color': '#e6a817', 'line-style': 'dashed',
                    'target-arrow-color': '#e6a817', 'target-arrow-shape': 'triangle',
                    'curve-style': 'bezier',
                    'label': function(ele) { return '\u26A1 ' + ele.data('label'); },
                    'font-size': 8, 'color': '#e6a817',
                    'text-rotation': 'autorotate', 'text-margin-y': -8,
                }
            },
        ],
        layout: { name: 'breadthfirst', directed: true, spacingFactor: 1.2, padding: 15 },
        minZoom: 0.3, maxZoom: 2,
        userPanningEnabled: false, userZoomingEnabled: false, boxSelectionEnabled: false,
    });

    miniCyInstance.on('tap', 'node', function(evt) {
        if (!evt.target.data('isCurrent')) showJobDetail(evt.target.id());
    });

    miniCyInstance.fit(undefined, 15);
}
