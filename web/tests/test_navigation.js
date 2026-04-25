// Tests for the 6-page navigation redesign
const fs = require('fs');
const path = require('path');

let passed = 0;
let failed = 0;

function assert(condition, msg) {
    if (condition) { passed++; }
    else { failed++; console.error('FAIL:', msg); }
}

// Read all view HTML files
const viewDir = path.join(__dirname, '..', 'partials', 'views');
const viewFiles = fs.readdirSync(viewDir).filter(f => f.endsWith('.html'));
const viewContents = {};
for (const f of viewFiles) {
    viewContents[f] = fs.readFileSync(path.join(viewDir, f), 'utf8');
}

// Read sidebar
const sidebar = fs.readFileSync(path.join(__dirname, '..', 'partials', 'sidebar.html'), 'utf8');

// Read index.html
const index = fs.readFileSync(path.join(__dirname, '..', 'index.html'), 'utf8');

// Read app.js
const appJs = fs.readFileSync(path.join(__dirname, '..', 'js', 'app.js'), 'utf8');

// Read tour.js
const tourJs = fs.readFileSync(path.join(__dirname, '..', 'js', 'tour.js'), 'utf8');

// --- Test: 6 main pages exist as view files ---
assert(viewContents['dashboard.html'], 'dashboard.html exists');
assert(viewContents['monitor.html'], 'monitor.html exists');
assert(viewContents['pipelines.html'], 'pipelines.html exists');
assert(viewContents['designer.html'], 'designer.html exists');
assert(viewContents['toolbox.html'], 'toolbox.html exists');
assert(viewContents['settings.html'], 'settings.html exists');
assert(viewContents['docs.html'], 'docs.html exists');

// --- Test: Each page has correct section ID ---
assert(viewContents['dashboard.html'].includes('id="dashboard-view"'), 'dashboard has dashboard-view section');
assert(viewContents['monitor.html'].includes('id="monitor-view"'), 'monitor has monitor-view section');
assert(viewContents['monitor.html'].includes('id="detail-view"'), 'monitor has detail-view section');
assert(viewContents['pipelines.html'].includes('id="pipelines-view"'), 'pipelines has pipelines-view section');
assert(viewContents['designer.html'].includes('id="designer-view"'), 'designer has designer-view section');
assert(viewContents['toolbox.html'].includes('id="toolbox-view"'), 'toolbox has toolbox-view section');
assert(viewContents['settings.html'].includes('id="settings-view"'), 'settings has settings-view section');

// --- Test: Monitor has 3 sub-tabs ---
assert(viewContents['monitor.html'].includes('id="st-monitor-jobs"'), 'monitor has jobs sub-tab button');
assert(viewContents['monitor.html'].includes('id="st-monitor-runs"'), 'monitor has runs sub-tab button');
assert(viewContents['monitor.html'].includes('id="st-monitor-events"'), 'monitor has events sub-tab button');
assert(viewContents['monitor.html'].includes('id="monitor-jobs-panel"'), 'monitor has jobs panel');
assert(viewContents['monitor.html'].includes('id="monitor-runs-panel"'), 'monitor has runs panel');
assert(viewContents['monitor.html'].includes('id="monitor-events-panel"'), 'monitor has events panel');

// --- Test: Pipelines has 2 sub-tabs ---
assert(viewContents['pipelines.html'].includes('id="st-pipelines-stages"'), 'pipelines has stages sub-tab');
assert(viewContents['pipelines.html'].includes('id="st-pipelines-map"'), 'pipelines has map sub-tab');
assert(viewContents['pipelines.html'].includes('id="pipelines-stages-panel"'), 'pipelines has stages panel');
assert(viewContents['pipelines.html'].includes('id="pipelines-map-panel"'), 'pipelines has map panel');

// --- Test: Toolbox has 3 sub-tabs ---
assert(viewContents['toolbox.html'].includes('id="st-toolbox-scripts"'), 'toolbox has scripts sub-tab');
assert(viewContents['toolbox.html'].includes('id="st-toolbox-variables"'), 'toolbox has variables sub-tab');
assert(viewContents['toolbox.html'].includes('id="st-toolbox-connections"'), 'toolbox has connections sub-tab');
assert(viewContents['toolbox.html'].includes('id="toolbox-scripts-panel"'), 'toolbox has scripts panel');
assert(viewContents['toolbox.html'].includes('id="toolbox-variables-panel"'), 'toolbox has variables panel');
assert(viewContents['toolbox.html'].includes('id="toolbox-connections-panel"'), 'toolbox has connections panel');

// --- Test: Key DOM IDs preserved across moves ---
assert(viewContents['monitor.html'].includes('id="search-input"'), 'search-input preserved in monitor');
assert(viewContents['monitor.html'].includes('id="jobs-table-wrap"'), 'jobs-table-wrap preserved in monitor');
assert(viewContents['monitor.html'].includes('id="jobs-pagination"'), 'jobs-pagination preserved in monitor');
assert(viewContents['monitor.html'].includes('id="all-execs-table-wrap"'), 'all-execs-table-wrap preserved in monitor');
assert(viewContents['monitor.html'].includes('id="events-list-wrap"'), 'events-list-wrap preserved in monitor');
assert(viewContents['monitor.html'].includes('id="exec-search-input"'), 'exec-search-input preserved in monitor');
assert(viewContents['monitor.html'].includes('id="event-search-input"'), 'event-search-input preserved in monitor');
assert(viewContents['monitor.html'].includes('id="detail-card"'), 'detail-card preserved in monitor');
assert(viewContents['pipelines.html'].includes('id="groups-grid"'), 'groups-grid preserved in pipelines');
assert(viewContents['pipelines.html'].includes('id="map-container"'), 'map-container preserved in pipelines');
assert(viewContents['toolbox.html'].includes('id="scripts-list-wrap"'), 'scripts-list-wrap preserved in toolbox');
assert(viewContents['toolbox.html'].includes('id="variables-table"'), 'variables-table preserved in toolbox');
assert(viewContents['toolbox.html'].includes('id="connections-list"'), 'connections-list preserved in toolbox');

// --- Test: Sidebar has exactly 6 nav tabs ---
const navTabs = sidebar.match(/id="tab-\w+"/g) || [];
assert(navTabs.length === 6, '6 nav tabs in sidebar (got ' + navTabs.length + ')');
assert(sidebar.includes('id="tab-dashboard"'), 'sidebar has dashboard tab');
assert(sidebar.includes('id="tab-monitor"'), 'sidebar has monitor tab');
assert(sidebar.includes('id="tab-pipelines"'), 'sidebar has pipelines tab');
assert(sidebar.includes('id="tab-designer"'), 'sidebar has designer tab');
assert(sidebar.includes('id="tab-toolbox"'), 'sidebar has toolbox tab');
assert(sidebar.includes('id="tab-settings"'), 'sidebar has settings tab');

// --- Test: Sidebar has docs ? button ---
assert(sidebar.includes('sidebar-help-btn'), 'sidebar has help/docs button');

// --- Test: Old tabs removed ---
assert(!sidebar.includes('id="tab-jobs"'), 'no tab-jobs in sidebar');
assert(!sidebar.includes('id="tab-executions"'), 'no tab-executions in sidebar');
assert(!sidebar.includes('id="tab-events"'), 'no tab-events in sidebar');
assert(!sidebar.includes('id="tab-scripts"'), 'no tab-scripts in sidebar');
assert(!sidebar.includes('id="tab-variables"'), 'no tab-variables in sidebar');
assert(!sidebar.includes('id="tab-connections"'), 'no tab-connections in sidebar');
assert(!sidebar.includes('id="tab-guide"'), 'no tab-guide in sidebar');

// --- Test: index.html includes new views ---
assert(index.includes('monitor.html'), 'index includes monitor.html');
assert(index.includes('pipelines.html'), 'index includes pipelines.html');
assert(index.includes('designer.html'), 'index includes designer.html');
assert(index.includes('toolbox.html'), 'index includes toolbox.html');

// --- Test: index.html does NOT include old views ---
assert(!index.includes('jobs.html'), 'index does not include old jobs.html');
assert(!index.includes('executions.html'), 'index does not include old executions.html');
assert(!index.includes('events.html'), 'index does not include old events.html');
assert(!index.includes('scripts.html'), 'index does not include old scripts.html');
assert(!index.includes('variables.html'), 'index does not include old variables.html');
assert(!index.includes('connections.html'), 'index does not include old connections.html');
assert(!index.includes('guide.html'), 'index does not include old guide.html');
assert(!index.includes('agents.html'), 'index does not include old agents.html');

// --- Test: ALL_VIEWS in app.js matches new structure ---
assert(appJs.includes("'dashboard','monitor','pipelines','designer','toolbox','settings','docs','detail'"), 'ALL_VIEWS has correct 8 entries');

// --- Test: PAGE_SUBTABS defined ---
assert(appJs.includes('PAGE_SUBTABS'), 'PAGE_SUBTABS defined in app.js');
assert(appJs.includes("monitor:"), 'PAGE_SUBTABS has monitor entry');
assert(appJs.includes("pipelines:"), 'PAGE_SUBTABS has pipelines entry');
assert(appJs.includes("toolbox:"), 'PAGE_SUBTABS has toolbox entry');

// --- Test: setSubTab function exists ---
assert(appJs.includes('function setSubTab('), 'setSubTab function defined');

// --- Test: Legacy route handling ---
assert(appJs.includes("parts[0] === 'jobs'"), 'handleRoute handles legacy #/jobs');
assert(appJs.includes("parts[0] === 'executions'"), 'handleRoute handles legacy #/executions');
assert(appJs.includes("parts[0] === 'events'"), 'handleRoute handles legacy #/events');
assert(appJs.includes("parts[0] === 'scripts'"), 'handleRoute handles legacy #/scripts');
assert(appJs.includes("parts[0] === 'variables'"), 'handleRoute handles legacy #/variables');
assert(appJs.includes("parts[0] === 'connections'"), 'handleRoute handles legacy #/connections');
assert(appJs.includes("parts[0] === 'agents'"), 'handleRoute handles legacy #/agents');

// --- Test: Tour references new tabs ---
assert(tourJs.includes('#tab-monitor'), 'tour references monitor tab');
assert(tourJs.includes('#tab-pipelines'), 'tour references pipelines tab');
assert(tourJs.includes('#tab-designer'), 'tour references designer tab');
assert(tourJs.includes('#tab-toolbox'), 'tour references toolbox tab');
assert(!tourJs.includes('#tab-jobs'), 'tour does not reference old jobs tab');
assert(!tourJs.includes('#tab-executions'), 'tour does not reference old executions tab');

// --- Test: Monitor has inline action bars (not global) ---
assert(viewContents['monitor.html'].includes('inline-action-bar'), 'monitor has inline action bars');
assert(viewContents['monitor.html'].includes('id="refresh-toggle"'), 'refresh-toggle in monitor');
assert(viewContents['monitor.html'].includes('id="refresh-countdown"'), 'refresh-countdown in monitor');

// --- Test: Back link in detail goes to monitor ---
assert(viewContents['monitor.html'].includes("showPage('monitor')"), 'detail back link goes to monitor');

console.log(passed + ' tests, ' + passed + ' passed, ' + failed + ' failed');
if (failed > 0) process.exit(1);
