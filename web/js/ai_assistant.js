// Kronforce — AI job generation assistant

let aiEnabled = false;

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
