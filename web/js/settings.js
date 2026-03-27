// Kronforce - Settings, keys, notification configuration
let currentSettingsTab = 'general';
const SETTINGS_TABS = ['general', 'auth', 'notifications'];

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

