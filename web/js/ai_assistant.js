// Kronforce — AI job generation assistant

let aiEnabled = false;
let aiPageJob = null;

function initAiPage() {
    const disabledMsg = document.getElementById('ai-page-disabled');
    const form = document.getElementById('ai-page-form');
    if (!disabledMsg || !form) return;
    if (!aiEnabled) {
        disabledMsg.style.display = '';
        const btn = document.getElementById('ai-page-btn');
        if (btn) btn.disabled = true;
        const prompt = document.getElementById('ai-page-prompt');
        if (prompt) prompt.disabled = true;
    } else {
        disabledMsg.style.display = 'none';
        const btn = document.getElementById('ai-page-btn');
        if (btn) btn.disabled = false;
        const prompt = document.getElementById('ai-page-prompt');
        if (prompt) prompt.disabled = false;
    }
}

async function aiPageGenerate() {
    const prompt = document.getElementById('ai-page-prompt').value.trim();
    const statusEl = document.getElementById('ai-page-status');
    const btn = document.getElementById('ai-page-btn');
    if (!prompt) { statusEl.textContent = 'Enter a description first'; statusEl.style.color = 'var(--danger)'; return; }

    btn.disabled = true;
    btn.textContent = 'Generating...';
    statusEl.textContent = 'Asking AI...';
    statusEl.style.color = 'var(--text-muted)';

    try {
        const result = await api('POST', '/api/ai/generate-job', { prompt });
        aiPageJob = result.job;
        renderAiPagePreview(result.job);
        document.getElementById('ai-page-result').style.display = '';
        statusEl.textContent = 'Job generated — review below.';
        statusEl.style.color = 'var(--success)';
    } catch (e) {
        statusEl.textContent = 'Error: ' + e.message;
        statusEl.style.color = 'var(--danger)';
    } finally {
        btn.disabled = false;
        btn.textContent = 'Generate Job';
    }
}

function renderAiPagePreview(job) {
    const el = document.getElementById('ai-page-preview');
    if (!el) return;
    let html = '<table style="width:100%;border-collapse:collapse">';
    const row = function(label, val) {
        return '<tr><td style="padding:4px 8px;font-weight:600;color:var(--text-primary);white-space:nowrap;vertical-align:top">' + esc(label) + '</td><td style="padding:4px 8px;color:var(--text-secondary)">' + val + '</td></tr>';
    };
    html += row('Name', esc(job.name || ''));
    html += row('Description', esc(job.description || ''));
    if (job.group) html += row('Group', esc(job.group));
    if (job.task) {
        html += row('Task Type', esc(job.task.type || ''));
        if (job.task.command) html += row('Command', '<code>' + esc(job.task.command) + '</code>');
        if (job.task.url) html += row('URL', esc(job.task.url));
        if (job.task.query) html += row('Query', '<code>' + esc(job.task.query) + '</code>');
        if (job.task.connection) html += row('Connection', esc(job.task.connection));
    }
    if (job.schedule) {
        if (job.schedule.type === 'cron') html += row('Schedule', 'Cron: <code>' + esc(job.schedule.value) + '</code>');
        else if (job.schedule.type === 'interval') html += row('Schedule', 'Every ' + (job.schedule.value.interval_secs || 0) + 's');
        else html += row('Schedule', esc(job.schedule.type));
    }
    if (job.timeout_secs) html += row('Timeout', job.timeout_secs + 's');
    if (job.retry_max) {
        let retryText = job.retry_max + ' times';
        if (job.retry_delay_secs) retryText += ', ' + job.retry_delay_secs + 's delay';
        if (job.retry_backoff > 1) retryText += ', ' + job.retry_backoff + 'x backoff';
        html += row('Retry', retryText);
    }
    if (job.max_concurrent) html += row('Concurrency', 'max ' + job.max_concurrent);
    if (job.priority) html += row('Priority', String(job.priority));
    if (job.approval_required) html += row('Approval', 'Required before execution');
    if (job.notifications) {
        const parts = [];
        if (job.notifications.on_failure) parts.push('on failure');
        if (job.notifications.on_success) parts.push('on success');
        if (job.notifications.on_assertion_failure) parts.push('on assertion failure');
        if (parts.length) html += row('Notifications', parts.join(', '));
    }
    if (job.output_rules) {
        const ruleParts = [];
        if (job.output_rules.extractions && job.output_rules.extractions.length) {
            ruleParts.push(job.output_rules.extractions.length + ' extraction' + (job.output_rules.extractions.length > 1 ? 's' : ''));
            for (const e of job.output_rules.extractions) {
                ruleParts.push('&nbsp;&nbsp;<code>' + esc(e.name) + '</code>: ' + esc(e.pattern) + (e.write_to_variable ? ' &rarr; <code>' + esc(e.write_to_variable) + '</code>' : ''));
            }
        }
        if (job.output_rules.assertions && job.output_rules.assertions.length) {
            ruleParts.push(job.output_rules.assertions.length + ' assertion' + (job.output_rules.assertions.length > 1 ? 's' : ''));
        }
        if (job.output_rules.triggers && job.output_rules.triggers.length) {
            ruleParts.push(job.output_rules.triggers.length + ' trigger' + (job.output_rules.triggers.length > 1 ? 's' : ''));
        }
        if (job.output_rules.forward_url) {
            ruleParts.push('Forward to: ' + esc(job.output_rules.forward_url));
        }
        if (ruleParts.length) html += row('Output Rules', ruleParts.join('<br>'));
    }
    if (job.parameters && job.parameters.length) {
        const paramText = job.parameters.map(function(p) {
            return '<code>' + esc(p.name) + '</code>' + (p.required ? ' *' : '') + (p.default ? ' = ' + esc(p.default) : '');
        }).join(', ');
        html += row('Parameters', paramText);
    }
    if (job.sla_deadline) html += row('SLA', job.sla_deadline + ' UTC' + (job.sla_warning_mins ? ' (warn ' + job.sla_warning_mins + 'm before)' : ''));
    html += '</table>';
    el.innerHTML = html;
}

function aiPageUseResult() {
    if (!aiPageJob) return;
    // Navigate to Builder and populate form with the AI result
    openCreateModal();
    setTimeout(function() { populateFormFromAiJob(aiPageJob); }, 200);
}

function populateFormFromAiJob(job) {
    if (job.name) document.getElementById('f-name').value = job.name;
    if (job.description) { var el = document.getElementById('f-desc'); if (el) el.value = job.description; }
    if (job.group) {
        var sel = document.getElementById('f-group');
        if (sel) { for (var i = 0; i < sel.options.length; i++) { if (sel.options[i].value.toLowerCase() === job.group.toLowerCase()) { sel.value = sel.options[i].value; break; } } }
    }
    if (job.task) {
        var radio = document.querySelector('input[name="task-type"][value="' + job.task.type + '"]');
        if (radio) { radio.checked = true; if (typeof updateTaskFields === 'function') updateTaskFields(); }
        if (job.task.type === 'shell' && job.task.command) { var el = document.getElementById('f-command'); if (el) el.value = job.task.command; }
        if (job.task.type === 'http') {
            if (job.task.url) { var el = document.getElementById('f-http-url'); if (el) el.value = job.task.url; }
            if (job.task.method) { var el = document.getElementById('f-http-method'); if (el) el.value = job.task.method; }
            if (job.task.expect_status) { var el = document.getElementById('f-http-expect'); if (el) el.value = job.task.expect_status; }
        }
        if (job.task.type === 'sql') {
            if (job.task.driver) { var el = document.getElementById('f-sql-driver'); if (el) el.value = job.task.driver; }
            if (job.task.query) { var el = document.getElementById('f-sql-query'); if (el) el.value = job.task.query; }
        }
    }
    if (job.schedule) {
        var schedRadio = document.querySelector('input[name="sched-type"][value="' + job.schedule.type + '"]');
        if (schedRadio) { schedRadio.checked = true; if (typeof updateSchedFields === 'function') updateSchedFields(); }
        if (job.schedule.type === 'cron' && job.schedule.value) {
            document.getElementById('f-cron').value = job.schedule.value;
            if (typeof parseCronToUI === 'function') parseCronToUI(job.schedule.value);
        }
    }
    if (job.timeout_secs) { var el = document.getElementById('f-timeout'); if (el) el.value = job.timeout_secs; }
    if (job.retry_max) { var el = document.getElementById('f-retry-max'); if (el) el.value = job.retry_max; }
    if (job.notifications) {
        if (job.notifications.on_failure) { var el = document.getElementById('f-notify-failure'); if (el) el.checked = true; }
        if (job.notifications.on_success) { var el = document.getElementById('f-notify-success'); if (el) el.checked = true; }
    }
}

async function checkAiEnabled() {
    try {
        const cfg = await (await fetch('/api/config')).json();
        aiEnabled = !!cfg.ai_enabled;
    } catch (e) {
        aiEnabled = false;
    }
}

function showAiPrompt() {
    const section = document.getElementById('ai-prompt-section');
    if (section && aiEnabled) {
        section.style.display = '';
        document.getElementById('ai-prompt-input').value = '';
        document.getElementById('ai-status').textContent = '';
    }
}

function hideAiPrompt() {
    const section = document.getElementById('ai-prompt-section');
    if (section) section.style.display = 'none';
}

async function aiGenerateJob() {
    const input = document.getElementById('ai-prompt-input');
    const statusEl = document.getElementById('ai-status');
    const btn = document.getElementById('ai-generate-btn');
    const prompt = input.value.trim();

    if (!prompt) {
        statusEl.textContent = 'Enter a description first';
        statusEl.style.color = 'var(--danger)';
        return;
    }

    btn.disabled = true;
    btn.textContent = 'Generating...';
    statusEl.textContent = 'Asking AI...';
    statusEl.style.color = 'var(--text-muted)';

    try {
        const result = await api('POST', '/api/ai/generate-job', { prompt: prompt });
        const job = result.job;

        // Populate form fields from AI response
        if (job.name) document.getElementById('f-name').value = job.name;
        if (job.description) document.getElementById('f-desc').value = job.description;
        if (job.group) {
            const groupSelect = document.getElementById('f-group');
            // Try to select the group, or leave as-is
            for (const opt of groupSelect.options) {
                if (opt.value.toLowerCase() === job.group.toLowerCase() || opt.text.toLowerCase() === job.group.toLowerCase()) {
                    groupSelect.value = opt.value;
                    break;
                }
            }
        }

        // Task type
        if (job.task) {
            const taskType = job.task.type;
            const radio = document.querySelector('input[name="task-type"][value="' + taskType + '"]');
            if (radio) {
                radio.checked = true;
                if (typeof updateTaskFields === 'function') updateTaskFields();
            }

            // Fill task-specific fields
            if (taskType === 'shell' && job.task.command) {
                const cmdEl = document.getElementById('f-command');
                if (cmdEl) cmdEl.value = job.task.command;
                if (job.task.working_dir) {
                    const wdEl = document.getElementById('f-working-dir');
                    if (wdEl) wdEl.value = job.task.working_dir;
                }
            } else if (taskType === 'http') {
                const urlEl = document.getElementById('f-http-url');
                if (urlEl && job.task.url) urlEl.value = job.task.url;
                const methodEl = document.getElementById('f-http-method');
                if (methodEl && job.task.method) methodEl.value = job.task.method;
                const expectEl = document.getElementById('f-http-expect-status');
                if (expectEl && job.task.expect_status) expectEl.value = job.task.expect_status;
                if (job.task.body) {
                    const bodyEl = document.getElementById('f-http-body');
                    if (bodyEl) bodyEl.value = job.task.body;
                }
            } else if (taskType === 'sql') {
                const driverEl = document.getElementById('f-sql-driver');
                if (driverEl && job.task.driver) driverEl.value = job.task.driver;
                const queryEl = document.getElementById('f-sql-query');
                if (queryEl && job.task.query) queryEl.value = job.task.query;
                const connStrEl = document.getElementById('f-sql-connection-string');
                if (connStrEl && job.task.connection_string) connStrEl.value = job.task.connection_string;
            }

            // Connection reference
            if (job.task.connection || job.connection) {
                const connEl = document.getElementById('f-task-connection');
                if (connEl) connEl.value = job.task.connection || job.connection;
            }
        }

        // Schedule
        if (job.schedule) {
            const schedType = job.schedule.type;
            const schedRadio = document.querySelector('input[name="sched-type"][value="' + schedType + '"]');
            if (schedRadio) {
                schedRadio.checked = true;
                if (typeof updateSchedFields === 'function') updateSchedFields();
            }

            if (schedType === 'cron' && job.schedule.value) {
                document.getElementById('f-cron').value = job.schedule.value;
                if (typeof parseCronToUI === 'function') parseCronToUI(job.schedule.value);
            } else if (schedType === 'interval' && job.schedule.value && job.schedule.value.interval_secs) {
                const intervalEl = document.getElementById('f-interval-secs');
                if (intervalEl) intervalEl.value = job.schedule.value.interval_secs;
            } else if (schedType === 'one_shot' && job.schedule.value) {
                const oneshotEl = document.getElementById('f-oneshot');
                if (oneshotEl) oneshotEl.value = job.schedule.value;
            }
        }

        // Advanced fields
        if (job.timeout_secs) {
            const el = document.getElementById('f-timeout');
            if (el) el.value = job.timeout_secs;
        }
        if (job.retry_max) {
            const el = document.getElementById('f-retry-max');
            if (el) el.value = job.retry_max;
        }
        if (job.retry_delay_secs) {
            const el = document.getElementById('f-retry-delay');
            if (el) el.value = job.retry_delay_secs;
        }
        if (job.notifications) {
            if (job.notifications.on_failure) {
                const el = document.getElementById('f-notify-failure');
                if (el) el.checked = true;
            }
            if (job.notifications.on_success) {
                const el = document.getElementById('f-notify-success');
                if (el) el.checked = true;
            }
        }
        if (job.approval_required) {
            const el = document.getElementById('f-approval-required');
            if (el) el.checked = true;
        }
        if (job.priority) {
            const el = document.getElementById('f-priority');
            if (el) el.value = job.priority;
        }
        if (job.max_concurrent) {
            const el = document.getElementById('f-max-concurrent');
            if (el) el.value = job.max_concurrent;
        }

        statusEl.textContent = 'Form filled — review and save.';
        statusEl.style.color = 'var(--success)';

    } catch (e) {
        statusEl.textContent = 'Error: ' + e.message;
        statusEl.style.color = 'var(--danger)';
    } finally {
        btn.disabled = false;
        btn.textContent = 'Generate';
    }
}
