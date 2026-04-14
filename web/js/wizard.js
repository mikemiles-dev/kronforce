// Kronforce - Setup wizard

var wizardStep = 0;
var wizardData = { jobCreated: null };
var WIZARD_STEPS = 5;

function showWizard() {
    wizardStep = 0;
    wizardData = { jobCreated: null };
    let html = '<div class="wizard-overlay" id="wizard-overlay" onclick="if(event.target===this)closeWizard()">';
    html += '<div class="wizard-card">';
    html += '<div class="wizard-header"><h2 id="wizard-title"></h2><div class="wizard-dots" id="wizard-dots"></div></div>';
    html += '<div class="wizard-body" id="wizard-body"></div>';
    html += '<div class="wizard-footer" id="wizard-footer"></div>';
    html += '</div></div>';
    document.body.insertAdjacentHTML('beforeend', html);
    renderWizardStep();
}

function closeWizard() {
    const el = document.getElementById('wizard-overlay');
    if (el) el.remove();
    // Mark completed
    api('PUT', '/api/settings', { wizard_completed: 'true' }).catch(() => {});
}

function wizardNext() { if (wizardStep < WIZARD_STEPS - 1) { wizardStep++; renderWizardStep(); } else { closeWizard(); } }
function wizardBack() { if (wizardStep > 0) { wizardStep--; renderWizardStep(); } }
function wizardSkip() { wizardNext(); }

function renderWizardStep() {
    const title = document.getElementById('wizard-title');
    const body = document.getElementById('wizard-body');
    const footer = document.getElementById('wizard-footer');
    const dots = document.getElementById('wizard-dots');

    // Dots
    let dotsHtml = '';
    for (let i = 0; i < WIZARD_STEPS; i++) {
        const cls = i === wizardStep ? 'active' : (i < wizardStep ? 'done' : '');
        dotsHtml += '<div class="wizard-dot ' + cls + '"></div>';
    }
    dots.innerHTML = dotsHtml;

    // Footer nav
    const backBtn = wizardStep > 0 ? '<button class="btn btn-ghost btn-sm" onclick="wizardBack()">Back</button>' : '<span></span>';
    const isLast = wizardStep === WIZARD_STEPS - 1;

    if (wizardStep === 0) {
        title.textContent = 'Welcome to Kronforce';
        body.innerHTML =
            '<p style="color:var(--text-secondary);line-height:1.6;margin-bottom:16px">A workload automation engine for scheduling jobs, managing agents, and building event-driven workflows.</p>' +
            '<div style="text-align:left;margin:0 auto;max-width:320px">' +
            '<p style="margin:8px 0;font-size:13px"><strong>&#128197; Job Scheduling</strong> — Cron, one-shot, on-demand, event triggers</p>' +
            '<p style="margin:8px 0;font-size:13px"><strong>&#128421; Distributed Agents</strong> — Push to standard agents, pull for custom agents</p>' +
            '<p style="margin:8px 0;font-size:13px"><strong>&#128220; Rhai Scripting</strong> — Custom logic with HTTP, shell, TCP/UDP built-ins</p>' +
            '<p style="margin:8px 0;font-size:13px"><strong>&#9889; Event Triggers</strong> — Chain jobs based on events and output patterns</p>' +
            '<p style="margin:8px 0;font-size:13px"><strong>&#128276; Notifications</strong> — Slack, Email, Teams, PagerDuty, SMS alerts on failures</p>' +
            '</div>';
        footer.innerHTML = '<span></span><button class="btn btn-primary btn-sm" onclick="wizardNext()">Let\'s get started</button>';
    } else if (wizardStep === 1) {
        title.textContent = 'Create Your First Job';
        body.innerHTML =
            '<p style="color:var(--text-secondary);margin-bottom:12px">Pick a template to get started quickly, or create from scratch.</p>' +
            '<div class="wizard-template-cards">' +
            '<div class="wizard-template-card" onclick="openTemplateJob(\'health-check\')">' +
            '<h4>&#128994; Health Check</h4><p>HTTP GET a URL every 5 minutes. Great for uptime monitoring.</p></div>' +
            '<div class="wizard-template-card" onclick="openTemplateJob(\'cron-task\')">' +
            '<h4>&#9200; Cron Task</h4><p>Run a shell command on a schedule. Backups, reports, cleanups.</p></div>' +
            '<div class="wizard-template-card" onclick="openTemplateJob(\'event-watcher\')">' +
            '<h4>&#9889; Event Watcher</h4><p>React to job failures. Alerting, cleanup, escalation.</p></div>' +
            '<div class="wizard-template-card" onclick="openTemplateJob(\'custom\')">' +
            '<h4>&#9881; Custom</h4><p>Open the full job creation form.</p></div>' +
            '</div>';
        footer.innerHTML = backBtn + '<button class="btn btn-ghost btn-sm" onclick="wizardSkip()">Skip</button>';
    } else if (wizardStep === 2) {
        title.textContent = 'Connect an Agent';
        const wizHost = getControllerUrl();
        body.innerHTML =
            '<p style="color:var(--text-secondary);margin-bottom:12px">Agents run jobs on remote machines. You can skip this if running everything locally.</p>' +
            '<div style="background:var(--bg-primary);border:1px solid var(--border);border-radius:var(--radius);padding:12px;margin-bottom:12px">' +
            '<p style="font-size:11px;color:var(--text-muted);margin:0 0 6px">Standard agent (binary):</p>' +
            '<pre style="font-size:11px;margin:0;white-space:pre-wrap;word-break:break-all">KRONFORCE_AGENT_KEY=&lt;your_agent_key&gt; \\\nKRONFORCE_CONTROLLER_URL=' + esc(wizHost) + ' \\\nKRONFORCE_AGENT_NAME=my-agent \\\n  ./kronforce-agent</pre>' +
            '</div>' +
            '<div style="background:var(--bg-primary);border:1px solid var(--border);border-radius:var(--radius);padding:12px;margin-bottom:12px">' +
            '<p style="font-size:11px;color:var(--text-muted);margin:0 0 6px">Docker:</p>' +
            '<pre style="font-size:11px;margin:0;white-space:pre-wrap;word-break:break-all">docker run -d \\\n  -e KRONFORCE_AGENT_KEY=&lt;your_agent_key&gt; \\\n  -e KRONFORCE_CONTROLLER_URL=' + esc(wizHost) + ' \\\n  -e KRONFORCE_AGENT_NAME=my-agent \\\n  ghcr.io/mikemiles-dev/kronforce:latest \\\n  kronforce-agent</pre>' +
            '</div>' +
            '<div style="background:var(--bg-primary);border:1px solid var(--border);border-radius:var(--radius);padding:12px;margin-bottom:12px">' +
            '<p style="font-size:11px;color:var(--text-muted);margin:0 0 6px">Custom agent (Python example):</p>' +
            '<pre style="font-size:11px;margin:0;white-space:pre-wrap;word-break:break-all">KRONFORCE_AGENT_KEY=&lt;your_agent_key&gt; \\\n  python3 examples/custom_agent.py</pre>' +
            '</div>' +
            '<div style="background:rgba(62,139,255,0.08);border:1px solid var(--accent);border-radius:var(--radius);padding:10px;font-size:11px;color:var(--text-secondary)">' +
            '<strong>Agent Key Required:</strong> Your agent key was printed to the console on first startup. You can also create new keys in <a href="#" onclick="closeWizard();showPage(\'settings\');return false" style="color:var(--accent)">Settings</a>.' +
            '</div>';
        footer.innerHTML = backBtn + '<button class="btn btn-ghost btn-sm" onclick="wizardSkip()">Skip</button>';
    } else if (wizardStep === 3) {
        title.textContent = 'Set Up Notifications';
        body.innerHTML =
            '<p style="color:var(--text-secondary);margin-bottom:12px">Get alerted when things go wrong. You can configure this later in Settings.</p>' +
            '<div class="form-group"><label>Email for alerts (optional)</label>' +
            '<input id="wizard-email" type="email" placeholder="ops@example.com"></div>' +
            '<label style="font-size:12px;display:flex;align-items:center;gap:4px"><input type="checkbox" id="wizard-agent-alert" checked> Alert when an agent goes offline</label>';
        footer.innerHTML = backBtn + '<div style="display:flex;gap:6px">' +
            '<button class="btn btn-ghost btn-sm" onclick="wizardSkip()">Skip</button>' +
            '<button class="btn btn-primary btn-sm" onclick="saveWizardNotifications()">Save & Continue</button></div>';
    } else if (wizardStep === 4) {
        title.textContent = 'You\'re All Set!';
        let summary = '<p style="color:var(--text-secondary);margin-bottom:16px">Kronforce is ready to go.</p>';
        summary += '<div style="text-align:left;margin:0 auto;max-width:320px">';
        summary += '<p style="margin:6px 0;font-size:13px">&#10004; Dashboard is live</p>';
        if (wizardData.jobCreated) summary += '<p style="margin:6px 0;font-size:13px">&#10004; Job \'' + esc(wizardData.jobCreated) + '\' created</p>';
        summary += '<p style="margin:6px 0;font-size:13px">&#10004; <a href="#" onclick="closeWizard();showPage(\'docs\');return false" style="color:var(--accent)">Browse the Docs</a> for detailed guides</p>';
        summary += '</div>';
        body.innerHTML = summary;
        footer.innerHTML = '<span></span><button class="btn btn-primary btn-sm" onclick="closeWizard()">Finish</button>';
    }
}

async function saveWizardNotifications() {
    const email = document.getElementById('wizard-email').value.trim();
    const agentAlert = document.getElementById('wizard-agent-alert').checked;
    try {
        const settings = {};
        if (email) {
            settings.notification_recipients = JSON.stringify({ emails: [email], phones: [] });
        }
        settings.notification_system_alerts = JSON.stringify({ agent_offline: agentAlert });
        await api('PUT', '/api/settings', settings);
        toast('Notifications saved');
    } catch (e) { /* ignore */ }
    wizardNext();
}

async function checkWizardNeeded() {
    try {
        const settings = await api('GET', '/api/settings');
        if (settings.wizard_completed) return;
        const res = await api('GET', '/api/jobs?per_page=1');
        if (res.total === 0) {
            showWizard();
        }
    } catch (e) { /* ignore — auth not set up yet, etc */ }
}
