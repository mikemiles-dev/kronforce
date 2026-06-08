// Kronforce - Settings, keys, notification configuration
let currentSettingsTab = 'general';

async function loadAiSettings() {
    try {
        const settings = await api('GET', '/api/settings');
        const keyEl = document.getElementById('settings-ai-key');
        const provEl = document.getElementById('settings-ai-provider');
        const modelEl = document.getElementById('settings-ai-model');
        const baseUrlEl = document.getElementById('settings-ai-base-url');
        const apiVersionEl = document.getElementById('settings-ai-api-version');
        if (keyEl && settings.ai_api_key) keyEl.value = settings.ai_api_key;
        // Map azure provider: stored as "openai" with a base_url, shown as "azure" in UI
        if (provEl && settings.ai_provider) {
            if (settings.ai_provider === 'openai' && settings.ai_base_url) {
                provEl.value = 'azure';
            } else {
                provEl.value = settings.ai_provider;
            }
        }
        if (baseUrlEl && settings.ai_base_url) baseUrlEl.value = settings.ai_base_url;
        if (apiVersionEl && settings.ai_api_version) apiVersionEl.value = settings.ai_api_version;
        toggleAzureFields();
        // Set model value after toggle (input may have been swapped)
        const modelEl2 = document.getElementById('settings-ai-model');
        if (modelEl2 && settings.ai_model) modelEl2.value = settings.ai_model;
        // Fetch models if key exists and not azure
        if (settings.ai_api_key && provEl.value !== 'azure') {
            await populateAiModelDropdown(settings.ai_model || '');
        }
    } catch (e) { /* ignore */ }
}

async function populateAiModelDropdown(selectedModel) {
    const modelEl = document.getElementById('settings-ai-model');
    if (!modelEl) return;
    try {
        const models = await api('GET', '/api/ai/models');
        const modelList = models.data || models.models || [];
        const names = modelList.map(function(m) { return m.id || m.name || ''; }).filter(Boolean).sort();
        modelEl.innerHTML = '<option value="">Auto-detect (' + (names[0] || 'default') + ')</option>';
        for (const name of names) {
            const selected = name === selectedModel ? ' selected' : '';
            modelEl.innerHTML += '<option value="' + esc(name) + '"' + selected + '>' + esc(name) + '</option>';
        }
    } catch (e) {
        modelEl.innerHTML = '<option value="">Auto-detect</option>';
        if (selectedModel) {
            modelEl.innerHTML += '<option value="' + esc(selectedModel) + '" selected>' + esc(selectedModel) + '</option>';
        }
    }
}

async function saveAiSettings() {
    const key = document.getElementById('settings-ai-key').value.trim();
    const providerUi = document.getElementById('settings-ai-provider').value;
    const model = document.getElementById('settings-ai-model').value.trim();
    const baseUrl = document.getElementById('settings-ai-base-url').value.trim();
    const apiVersion = document.getElementById('settings-ai-api-version').value.trim();
    const statusEl = document.getElementById('ai-settings-status');

    // Azure is stored as provider "openai" with a base_url
    const provider = providerUi === 'azure' ? 'openai' : providerUi;

    try {
        const body = {};
        body.ai_api_key = key;
        body.ai_provider = provider;
        if (model) body.ai_model = model;
        else body.ai_model = '';
        body.ai_base_url = providerUi === 'azure' ? baseUrl : '';
        body.ai_api_version = providerUi === 'azure' ? (apiVersion || 'preview') : '';
        await api('PUT', '/api/settings', body);
        aiEnabled = !!key;
        if (typeof initAiPage === 'function') initAiPage();

        if (key) {
            statusEl.textContent = 'Saved. Loading models...';
            statusEl.style.color = 'var(--text-muted)';
            await populateAiModelDropdown(model);
            statusEl.textContent = 'Saved';
            statusEl.style.color = 'var(--success)';
        } else {
            statusEl.textContent = 'Saved';
            statusEl.style.color = 'var(--success)';
        }
        setTimeout(function() { statusEl.textContent = ''; }, 5000);
    } catch (e) {
        statusEl.textContent = 'Error: ' + e.message;
        statusEl.style.color = 'var(--danger)';
    }
}

function toggleSettingsAiKeyVis() {
    const el = document.getElementById('settings-ai-key');
    if (el) el.type = el.type === 'password' ? 'text' : 'password';
}

function toggleAzureFields() {
    const provider = document.getElementById('settings-ai-provider').value;
    const azureFields = document.getElementById('azure-ai-fields');
    if (azureFields) azureFields.style.display = provider === 'azure' ? '' : 'none';
    // Swap model field: text input for azure, select for others
    const wrap = document.getElementById('settings-ai-model-wrap');
    if (!wrap) return;
    const current = wrap.querySelector('#settings-ai-model');
    const currentVal = current ? current.value : '';
    if (provider === 'azure') {
        if (current && current.tagName === 'SELECT') {
            wrap.innerHTML = '<input id="settings-ai-model" type="text" placeholder="deployment name (e.g. gpt-4o)" style="width:100%;font-size:12px" value="' + esc(currentVal) + '">';
        }
    } else {
        if (current && current.tagName === 'INPUT') {
            wrap.innerHTML = '<select id="settings-ai-model" style="width:100%;font-size:12px"><option value="">Auto-detect</option></select>';
        }
    }
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
            html += '<div class="card">';
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

async function vacuumDatabase() {
    const btn = document.getElementById('vacuum-btn');
    const status = document.getElementById('vacuum-status');
    if (!confirm('Run VACUUM now? Concurrent writes will be briefly blocked.')) return;
    btn.disabled = true;
    status.style.color = 'var(--text-muted)';
    status.textContent = 'Running...';
    try {
        const res = await api('POST', '/api/admin/vacuum');
        const fmt = b => (b / (1024 * 1024)).toFixed(1) + ' MB';
        const delta = res.size_before - res.size_after;
        const sign = delta >= 0 ? '−' : '+';
        status.style.color = 'var(--success)';
        status.textContent = 'Done in ' + (res.elapsed_ms / 1000).toFixed(1) + 's: '
            + fmt(res.size_before) + ' → ' + fmt(res.size_after)
            + ' (' + sign + fmt(Math.abs(delta)) + ')';
    } catch (e) {
        status.style.color = 'var(--danger)';
        status.textContent = 'Error: ' + e.message;
    } finally {
        btn.disabled = false;
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

