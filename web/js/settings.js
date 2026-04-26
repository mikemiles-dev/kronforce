// Kronforce - Settings, keys, notification configuration
let currentSettingsTab = 'general';

async function loadAiSettings() {
    try {
        const settings = await api('GET', '/api/settings');
        const keyEl = document.getElementById('settings-ai-key');
        const provEl = document.getElementById('settings-ai-provider');
        const modelEl = document.getElementById('settings-ai-model');
        if (keyEl && settings.ai_api_key) keyEl.value = settings.ai_api_key;
        if (provEl && settings.ai_provider) provEl.value = settings.ai_provider;
        if (modelEl && settings.ai_model) modelEl.value = settings.ai_model;
    } catch (e) { /* ignore */ }
}

async function saveAiSettings() {
    const key = document.getElementById('settings-ai-key').value.trim();
    const provider = document.getElementById('settings-ai-provider').value;
    const model = document.getElementById('settings-ai-model').value.trim();
    const statusEl = document.getElementById('ai-settings-status');
    try {
        const body = {};
        body.ai_api_key = key;
        body.ai_provider = provider;
        if (model) body.ai_model = model;
        else body.ai_model = '';
        await api('PUT', '/api/settings', body);
        statusEl.textContent = 'Saved';
        statusEl.style.color = 'var(--success)';
        // Update global flag
        aiEnabled = !!key;
        if (typeof initAiPage === 'function') initAiPage();
        setTimeout(function() { statusEl.textContent = ''; }, 3000);
    } catch (e) {
        statusEl.textContent = 'Error: ' + e.message;
        statusEl.style.color = 'var(--danger)';
    }
}

function toggleSettingsAiKeyVis() {
    const el = document.getElementById('settings-ai-key');
    if (el) el.type = el.type === 'password' ? 'text' : 'password';
}
const SETTINGS_TABS = ['general', 'auth', 'notifications', 'agents'];

function showSettingsTab(tab) {
    currentSettingsTab = tab;
    SETTINGS_TABS.forEach(t => {
        const panel = document.getElementById('settings-panel-' + t);
        const btn = document.getElementById('stab-' + t);
        if (panel) panel.style.display = t === tab ? '' : 'none';
        if (btn) {
            btn.classList.toggle('active', t === tab);
            btn.style.background = t === tab ? 'var(--surface)' : '';
            btn.style.borderBottom = t === tab ? '2px solid var(--accent)' : '';
        }
    });
    if (tab === 'agents') renderSettingsAgents();
}

async function renderSettingsAgents() {
    const wrap = document.getElementById('settings-agents-wrap');
    if (!wrap) return;
    try {
        const agents = await api('GET', '/api/agents');
        if (agents.length === 0) {
            wrap.innerHTML = '<div class="card"><div style="padding:24px;text-align:center;color:var(--text-muted)">No agents registered. Start an agent to see it here.</div></div>';
            return;
        }
        let html = '<div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(280px,1fr));gap:12px">';
        for (const a of agents) {
            const online = a.status === 'online';
            const dot = online ? '<span style="color:var(--success)">&#9679;</span>' : '<span style="color:var(--text-muted)">&#9679;</span>';
            const typeBadge = a.agent_type === 'custom' ? ' <span class="badge badge-paused" style="font-size:9px">custom</span>' : '';
            const tags = (a.tags || []).map(t => '<span style="font-size:10px;background:var(--bg-tertiary);padding:1px 6px;border-radius:8px">' + esc(t) + '</span>').join(' ');
            html += '<div class="card" style="cursor:pointer" onclick="showPage(\'agents\')">';
            html += '<div style="padding:14px">';
            html += '<div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:6px">';
            html += '<strong>' + dot + ' ' + esc(a.name) + typeBadge + '</strong>';
            html += '<span style="font-size:11px;color:var(--text-muted)">' + esc(a.hostname || '') + '</span>';
            html += '</div>';
            if (tags) html += '<div style="display:flex;gap:4px;flex-wrap:wrap;margin-bottom:4px">' + tags + '</div>';
            html += '<div style="font-size:11px;color:var(--text-muted)">' + esc(a.address || '') + ':' + (a.port || '') + '</div>';
            if (a.last_heartbeat) html += '<div style="font-size:11px;color:var(--text-muted)">heartbeat ' + fmtDate(a.last_heartbeat) + '</div>';
            html += '</div></div>';
        }
        html += '</div>';
        wrap.innerHTML = html;
    } catch (e) {
        wrap.innerHTML = '<div class="card"><div style="padding:16px;color:var(--danger)">Failed to load agents</div></div>';
    }
}

async function loadRetention() {
    try {
        const settings = await api('GET', '/api/settings');
        const val = settings.retention_days || '7';
        document.getElementById('retention-days').value = val;
    } catch (e) {
        // Settings may not be available if auth is disabled during first setup
    }
}

async function saveRetention() {
    const val = document.getElementById('retention-days').value;
    const status = document.getElementById('retention-status');
    try {
        await api('PUT', '/api/settings', { retention_days: val });
        status.textContent = 'Saved';
        status.style.color = 'var(--success)';
        setTimeout(() => { status.textContent = ''; }, 2000);
    } catch (e) {
        status.textContent = 'Error: ' + e.message;
        status.style.color = 'var(--danger)';
    }
}

async function loadNotificationSettings() {
    try {
        const settings = await api('GET', '/api/settings');
        const email = settings.notification_email ? JSON.parse(settings.notification_email) : {};
        document.getElementById('notif-email-enabled').checked = email.enabled || false;
        document.getElementById('notif-smtp-host').value = email.smtp_host || '';
        document.getElementById('notif-smtp-port').value = email.smtp_port || '';
        document.getElementById('notif-smtp-user').value = email.username || '';
        document.getElementById('notif-smtp-pass').value = email.password || '';
        document.getElementById('notif-smtp-from').value = email.from || '';
        document.getElementById('notif-smtp-tls').checked = email.tls !== false;

        const sms = settings.notification_sms ? JSON.parse(settings.notification_sms) : {};
        document.getElementById('notif-sms-enabled').checked = sms.enabled || false;
        document.getElementById('notif-sms-url').value = sms.webhook_url || '';
        document.getElementById('notif-sms-user').value = sms.auth_user || '';
        document.getElementById('notif-sms-pass').value = sms.auth_pass || '';
        document.getElementById('notif-sms-from').value = sms.from_number || '';

        const webhook = settings.notification_webhook ? JSON.parse(settings.notification_webhook) : {};
        document.getElementById('notif-webhook-enabled').checked = webhook.enabled || false;
        document.getElementById('notif-webhook-url').value = webhook.url || '';
        document.getElementById('notif-webhook-format').value = webhook.format || 'slack';
        document.getElementById('notif-webhook-headers').value = webhook.headers && Object.keys(webhook.headers).length ? JSON.stringify(webhook.headers) : '';

        const recipients = settings.notification_recipients ? JSON.parse(settings.notification_recipients) : {};
        document.getElementById('notif-emails').value = (recipients.emails || []).join('\n');
        document.getElementById('notif-phones').value = (recipients.phones || []).join('\n');

        const alerts = settings.notification_system_alerts ? JSON.parse(settings.notification_system_alerts) : {};
        document.getElementById('notif-alert-agent-offline').checked = alerts.agent_offline || false;
    } catch (e) { /* settings may not be loaded yet */ }
}

async function saveNotificationSettings() {
    const status = document.getElementById('notif-status');
    try {
        const settings = {};
        settings.notification_email = JSON.stringify({
            enabled: document.getElementById('notif-email-enabled').checked,
            smtp_host: document.getElementById('notif-smtp-host').value.trim(),
            smtp_port: parseInt(document.getElementById('notif-smtp-port').value) || 587,
            username: document.getElementById('notif-smtp-user').value.trim(),
            password: document.getElementById('notif-smtp-pass').value,
            from: document.getElementById('notif-smtp-from').value.trim(),
            tls: document.getElementById('notif-smtp-tls').checked,
        });
        settings.notification_sms = JSON.stringify({
            enabled: document.getElementById('notif-sms-enabled').checked,
            webhook_url: document.getElementById('notif-sms-url').value.trim(),
            auth_user: document.getElementById('notif-sms-user').value.trim() || null,
            auth_pass: document.getElementById('notif-sms-pass').value || null,
            from_number: document.getElementById('notif-sms-from').value.trim() || null,
        });
        let webhookHeaders = {};
        try { const h = document.getElementById('notif-webhook-headers').value.trim(); if (h) webhookHeaders = JSON.parse(h); } catch(e) { /* ignore invalid JSON */ }
        settings.notification_webhook = JSON.stringify({
            enabled: document.getElementById('notif-webhook-enabled').checked,
            url: document.getElementById('notif-webhook-url').value.trim(),
            format: document.getElementById('notif-webhook-format').value,
            headers: webhookHeaders,
        });
        settings.notification_recipients = JSON.stringify({
            emails: document.getElementById('notif-emails').value.split('\n').map(s => s.trim()).filter(Boolean),
            phones: document.getElementById('notif-phones').value.split('\n').map(s => s.trim()).filter(Boolean),
        });
        settings.notification_system_alerts = JSON.stringify({
            agent_offline: document.getElementById('notif-alert-agent-offline').checked,
        });
        await api('PUT', '/api/settings', settings);
        status.textContent = 'Saved';
        status.style.color = 'var(--success)';
        setTimeout(() => { status.textContent = ''; }, 2000);
    } catch (e) {
        status.textContent = 'Error: ' + e.message;
        status.style.color = 'var(--danger)';
    }
}

async function testNotification() {
    const status = document.getElementById('notif-status');
    status.textContent = 'Sending...';
    status.style.color = 'var(--text-muted)';
    try {
        const res = await api('POST', '/api/notifications/test');
        status.textContent = res.message || 'Done';
        status.style.color = res.status === 'ok' ? 'var(--success)' : 'var(--danger)';
        setTimeout(() => { status.textContent = ''; }, 5000);
    } catch (e) {
        status.textContent = 'Error: ' + e.message;
        status.style.color = 'var(--danger)';
    }
}

