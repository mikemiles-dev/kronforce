// Kronforce - Create/edit modal, cron builder, extraction rows, dependency maps, notifications, setup wizard

function populateGroupSelect(selectedGroup) {
    const sel = document.getElementById('f-group');
    if (!sel) return;
    const isDefault = !selectedGroup || selectedGroup === 'Default';
    sel.innerHTML = '<option value=""' + (isDefault ? ' selected' : '') + '>Default</option>';
    for (const g of cachedGroups) {
        if (g === 'Default') continue; // already added as first option
        const selected = g === selectedGroup ? ' selected' : '';
        sel.innerHTML += '<option value="' + esc(g) + '"' + selected + '>' + esc(g) + '</option>';
    }
    if (selectedGroup && selectedGroup !== 'Default' && !cachedGroups.includes(selectedGroup)) {
        sel.innerHTML += '<option value="' + esc(selectedGroup) + '" selected>' + esc(selectedGroup) + '</option>';
    }
}

// --- Create/Edit Modal ---
function openCreateModal() {
    editingJobId = null;
    resetJobTabs();
    filePushBase64 = '';
    filePushFilename = '';
    filePushSize = 0;
    selectedCustomAgentData = null;
    document.getElementById('modal-title').textContent = 'Create Job';
    document.getElementById('f-name').value = '';
    document.getElementById('f-command').value = '';
    document.getElementById('f-run-as').value = '';
    populateTaskForm(null);
    parseCronToUI('');
    document.getElementById('f-desc').value = '';
    populateGroupSelect('');
    document.getElementById('f-retry-max').value = '0';
    document.getElementById('f-retry-delay').value = '0';
    document.getElementById('f-retry-backoff').value = '1.0';
    document.getElementById('f-cron').value = '';
    document.getElementById('f-oneshot').value = '';
    document.getElementById('f-timeout').value = '';
    document.querySelector('input[name="sched-type"][value="one_shot"]').checked = true;
    document.getElementById('f-oneshot').value = toLocalDatetimeString(new Date());
    updateSchedFields();
    document.querySelector('input[name="exec-mode"][value="standard"]').checked = true;
    currentExecMode = 'standard';
    document.querySelector('input[name="target-type"][value="local"]').checked = true;
    updateExecutionMode();
    updateTargetFields();
    populateDeps(null);
    populateOutputRules(null);
    populateJobNotifications(null);
    openModal('create-modal');
}

async function copyJob(id) {
    try {
        const job = await api('GET', '/api/jobs/' + id);
        // Open as a new job with copied data
        openCreateModal();
        document.getElementById('modal-title').textContent = 'Copy Job';
        document.getElementById('f-name').value = job.name + '-copy';

        // Populate task
        if (job.task.type === 'custom') {
            document.querySelector('input[name="exec-mode"][value="custom"]').checked = true;
            currentExecMode = 'custom';
            updateExecutionMode();
            await populateCustomAgentSelect();
            if (job.target && job.target.agent_id) {
                document.getElementById('f-custom-agent').value = job.target.agent_id;
                await onCustomAgentSelected();
                const radio = document.querySelector('input[name="custom-task-type"][value="' + job.task.agent_task_type + '"]');
                if (radio) { radio.checked = true; onCustomTaskTypeSelected(); }
                if (job.task.data) {
                    for (const [key, val] of Object.entries(job.task.data)) {
                        const el = document.getElementById('f-custom-' + key);
                        if (el) el.value = val;
                    }
                }
            }
        } else {
            document.querySelector('input[name="exec-mode"][value="standard"]').checked = true;
            currentExecMode = 'standard';
            updateExecutionMode();
            populateTaskForm(job.task);
        }

        // Populate schedule
        document.getElementById('f-desc').value = job.description || '';
        populateGroupSelect(job.group || '');
        document.getElementById('f-retry-max').value = job.retry_max || 0;
        document.getElementById('f-retry-delay').value = job.retry_delay_secs || 0;
        document.getElementById('f-retry-backoff').value = job.retry_backoff || 1.0;
        document.getElementById('f-run-as').value = job.run_as || '';
        document.getElementById('f-timeout').value = job.timeout_secs || '';

        let schedType = job.schedule.type;
        if (schedType === 'manual') schedType = 'on_demand';
        document.querySelector('input[name="sched-type"][value="' + schedType + '"]').checked = true;
        updateSchedFields();
        if (schedType === 'cron') {
            document.getElementById('f-cron').value = job.schedule.value;
            parseCronToUI(job.schedule.value);
        } else if (schedType === 'one_shot') {
            document.getElementById('f-oneshot').value = toLocalDatetimeString(new Date());
        } else if (schedType === 'event' && job.schedule.value) {
            setEventKindValue(job.schedule.value.kind_pattern || '*');
            document.getElementById('f-event-severity').value = job.schedule.value.severity || '';
            document.getElementById('f-event-job-filter').value = job.schedule.value.job_name_filter || '';
        }

        // Populate target
        const target = job.target;
        if (target && target.type === 'agent' && job.task.type !== 'custom') {
            document.querySelector('input[name="target-type"][value="agent"]').checked = true;
            updateTargetFields();
            await populateAgentSelect('standard');
            document.getElementById('f-agent').value = target.agent_id;
        } else if (target && target.type === 'any') {
            document.querySelector('input[name="target-type"][value="any"]').checked = true;
        } else if (target && target.type === 'all') {
            document.querySelector('input[name="target-type"][value="all"]').checked = true;
        }

        // Populate advanced
        populateDeps(null, job.depends_on);
        populateOutputRules(job.output_rules);
        populateJobNotifications(job.notifications);
    } catch (e) {
        toast(e.message, 'error');
    }
}

async function openEditModal(id) {
    try {
        const job = await api('GET', '/api/jobs/' + id);
        editingJobId = id;
        document.getElementById('modal-title').textContent = 'Edit Job';
        document.getElementById('f-name').value = job.name;

        // Determine execution mode from task type
        if (job.task.type === 'custom') {
            document.querySelector('input[name="exec-mode"][value="custom"]').checked = true;
            currentExecMode = 'custom';
            updateExecutionMode();
            // Populate custom agent and task type
            await populateCustomAgentSelect();
            if (job.target && job.target.agent_id) {
                document.getElementById('f-custom-agent').value = job.target.agent_id;
                await onCustomAgentSelected();
                // Select the right custom task type
                const radio = document.querySelector('input[name="custom-task-type"][value="' + job.task.agent_task_type + '"]');
                if (radio) { radio.checked = true; onCustomTaskTypeSelected(); }
                // Fill in field values
                if (job.task.data) {
                    for (const [key, val] of Object.entries(job.task.data)) {
                        const el = document.getElementById('f-custom-' + key);
                        if (el) el.value = val;
                    }
                }
            }
        } else {
            document.querySelector('input[name="exec-mode"][value="standard"]').checked = true;
            currentExecMode = 'standard';
            updateExecutionMode();
            populateTaskForm(job.task);
        }

        document.getElementById('f-run-as').value = job.run_as || '';
        document.getElementById('f-desc').value = job.description || '';
        populateGroupSelect(job.group || '');
        document.getElementById('f-retry-max').value = job.retry_max || 0;
        document.getElementById('f-retry-delay').value = job.retry_delay_secs || 0;
        document.getElementById('f-retry-backoff').value = job.retry_backoff || 1.0;
        document.getElementById('f-timeout').value = job.timeout_secs || '';

        let schedType = job.schedule.type;
        if (schedType === 'manual') schedType = 'on_demand';
        document.querySelector('input[name="sched-type"][value="' + schedType + '"]').checked = true;
        updateSchedFields();

        if (schedType === 'cron') {
            document.getElementById('f-cron').value = job.schedule.value;
            parseCronToUI(job.schedule.value);
        } else if (schedType === 'one_shot') {
            const dt = new Date(job.schedule.value);
            document.getElementById('f-oneshot').value = toLocalDatetimeString(dt);
        } else if (schedType === 'event' && job.schedule.value) {
            setEventKindValue(job.schedule.value.kind_pattern || '*');
            document.getElementById('f-event-severity').value = job.schedule.value.severity || '';
            document.getElementById('f-event-job-filter').value = job.schedule.value.job_name_filter || '';
        }

        // Set target
        const target = job.target;
        if (!target || target.type === 'local') {
            document.querySelector('input[name="target-type"][value="local"]').checked = true;
        } else if (target.type === 'agent') {
            document.querySelector('input[name="target-type"][value="agent"]').checked = true;
        } else if (target.type === 'any') {
            document.querySelector('input[name="target-type"][value="any"]').checked = true;
        } else if (target.type === 'all') {
            document.querySelector('input[name="target-type"][value="all"]').checked = true;
        }
        updateTargetFields();
        if (target && target.type === 'agent') {
            await populateAgentSelect();
            document.getElementById('f-agent').value = target.agent_id;
        }

        populateDeps(id, job.depends_on);
        populateOutputRules(job.output_rules);
        populateJobNotifications(job.notifications);
        openModal('create-modal');
    } catch (e) {
        toast(e.message, 'error');
    }
}

function closeCreateModal() {
    closeModal('create-modal');
}

function updateTaskFields() {
    const type = document.querySelector('input[name="task-type"]:checked').value;
    document.getElementById('task-shell-fields').style.display = type === 'shell' ? '' : 'none';
    document.getElementById('task-http-fields').style.display = type === 'http' ? '' : 'none';
    document.getElementById('task-sql-fields').style.display = type === 'sql' ? '' : 'none';
    document.getElementById('task-ftp-fields').style.display = type === 'ftp' ? '' : 'none';
    document.getElementById('task-script-fields').style.display = type === 'script' ? '' : 'none';
    document.getElementById('task-filepush-fields').style.display = type === 'file_push' ? '' : 'none';
    document.getElementById('task-kafka-fields').style.display = type === 'kafka' ? '' : 'none';
    document.getElementById('task-rabbitmq-fields').style.display = type === 'rabbitmq' ? '' : 'none';
    document.getElementById('task-mqtt-fields').style.display = type === 'mqtt' ? '' : 'none';
    document.getElementById('task-redis-fields').style.display = type === 'redis' ? '' : 'none';
    document.getElementById('task-mcp-fields').style.display = type === 'mcp' ? '' : 'none';
    if (type === 'script') populateScriptDropdown();
}

let filePushBase64 = '';
let filePushFilename = '';
let filePushSize = 0;

function onFilePushFileSelected(input) {
    const file = input.files[0];
    const info = document.getElementById('f-filepush-info');
    if (!file) { info.style.display = 'none'; filePushBase64 = ''; return; }
    if (file.size > 5 * 1024 * 1024) {
        toast('File exceeds 5MB limit', 'error');
        input.value = '';
        info.style.display = 'none';
        filePushBase64 = '';
        return;
    }
    filePushFilename = file.name;
    filePushSize = file.size;
    const reader = new FileReader();
    reader.onload = () => {
        filePushBase64 = btoa(reader.result);
        info.textContent = file.name + ' (' + (file.size / 1024).toFixed(1) + ' KB)';
        info.style.display = '';
    };
    reader.readAsBinaryString(file);
}

function buildTaskFromForm() {
    const type = document.querySelector('input[name="task-type"]:checked').value;
    if (type === 'shell') {
        const command = document.getElementById('f-command').value.trim();
        if (!command) return null;
        return { type: 'shell', command };
    }
    if (type === 'http') {
        const url = document.getElementById('f-http-url').value.trim();
        if (!url) return null;
        const task = { type: 'http', method: document.getElementById('f-http-method').value, url };
        const hdrs = document.getElementById('f-http-headers').value.trim();
        if (hdrs) { try { task.headers = JSON.parse(hdrs); } catch(e) { toast('Invalid headers JSON', 'error'); return null; } }
        const body = document.getElementById('f-http-body').value.trim();
        if (body) task.body = body;
        const expect = document.getElementById('f-http-expect').value.trim();
        if (expect) task.expect_status = parseInt(expect);
        return task;
    }
    if (type === 'sql') {
        const query = document.getElementById('f-sql-query').value.trim();
        const conn = document.getElementById('f-sql-conn').value.trim();
        if (!query || !conn) return null;
        return { type: 'sql', driver: document.getElementById('f-sql-driver').value, connection_string: conn, query };
    }
    if (type === 'ftp') {
        const host = document.getElementById('f-ftp-host').value.trim();
        const remote = document.getElementById('f-ftp-remote').value.trim();
        const local = document.getElementById('f-ftp-local').value.trim();
        if (!host || !remote || !local) return null;
        const port = document.getElementById('f-ftp-port').value.trim();
        return {
            type: 'ftp',
            protocol: document.getElementById('f-ftp-proto').value,
            host, port: port ? parseInt(port) : null,
            username: document.getElementById('f-ftp-user').value.trim(),
            password: document.getElementById('f-ftp-pass').value.trim(),
            direction: document.getElementById('f-ftp-dir').value,
            remote_path: remote, local_path: local,
        };
    }
    if (type === 'script') {
        const scriptName = document.getElementById('f-script-name').value;
        if (!scriptName) return null;
        return { type: 'script', script_name: scriptName };
    }
    if (type === 'file_push') {
        const dest = document.getElementById('f-filepush-dest').value.trim();
        if (!dest) { toast('Destination path is required', 'error'); return null; }
        if (!filePushBase64) { toast('Select a file to upload', 'error'); return null; }
        const perms = document.getElementById('f-filepush-perms').value.trim();
        return {
            type: 'file_push',
            filename: filePushFilename,
            destination: dest,
            content_base64: filePushBase64,
            permissions: perms || null,
            overwrite: document.getElementById('f-filepush-overwrite').checked,
        };
    }
    if (type === 'kafka') {
        const broker = document.getElementById('f-kafka-broker').value.trim();
        const topic = document.getElementById('f-kafka-topic').value.trim();
        const message = document.getElementById('f-kafka-message').value;
        if (!broker || !topic || !message) return null;
        const task = { type: 'kafka', broker, topic, message };
        const key = document.getElementById('f-kafka-key').value.trim();
        if (key) task.key = key;
        const props = document.getElementById('f-kafka-props').value.trim();
        if (props) task.properties = props;
        return task;
    }
    if (type === 'rabbitmq') {
        const url = document.getElementById('f-rabbitmq-url').value.trim();
        const exchange = document.getElementById('f-rabbitmq-exchange').value.trim();
        const routing_key = document.getElementById('f-rabbitmq-routing').value.trim();
        const message = document.getElementById('f-rabbitmq-message').value;
        if (!url || !exchange || !routing_key || !message) return null;
        const task = { type: 'rabbitmq', url, exchange, routing_key, message };
        const ct = document.getElementById('f-rabbitmq-ctype').value.trim();
        if (ct) task.content_type = ct;
        return task;
    }
    if (type === 'mqtt') {
        const broker = document.getElementById('f-mqtt-broker').value.trim();
        const topic = document.getElementById('f-mqtt-topic').value.trim();
        const message = document.getElementById('f-mqtt-message').value;
        if (!broker || !topic || !message) return null;
        const task = { type: 'mqtt', broker, topic, message };
        const port = document.getElementById('f-mqtt-port').value.trim();
        if (port) task.port = parseInt(port);
        task.qos = parseInt(document.getElementById('f-mqtt-qos').value);
        const user = document.getElementById('f-mqtt-user').value.trim();
        if (user) task.username = user;
        const pass = document.getElementById('f-mqtt-pass').value;
        if (pass) task.password = pass;
        const cid = document.getElementById('f-mqtt-clientid').value.trim();
        if (cid) task.client_id = cid;
        return task;
    }
    if (type === 'redis') {
        const url = document.getElementById('f-redis-url').value.trim();
        const channel = document.getElementById('f-redis-channel').value.trim();
        const message = document.getElementById('f-redis-message').value;
        if (!url || !channel || !message) return null;
        return { type: 'redis', url, channel, message };
    }
    if (type === 'mcp') {
        const server = document.getElementById('f-mcp-server').value.trim();
        const tool = document.getElementById('f-mcp-tool').value.trim();
        if (!server || !tool) return null;
        const task = { type: 'mcp', server_url: server, tool };
        const args = document.getElementById('f-mcp-args').value.trim();
        if (args) { try { task.arguments = JSON.parse(args); } catch(e) { toast('Invalid arguments JSON', 'error'); return null; } }
        return task;
    }
    return null;
}

function buildCustomTaskFromForm() {
    if (!selectedCustomAgentData) return null;
    const typeName = document.querySelector('input[name="custom-task-type"]:checked')?.value;
    if (!typeName) return null;
    const taskDef = selectedCustomAgentData.task_types.find(t => t.name === typeName);
    if (!taskDef) return null;
    const data = {};
    for (const f of taskDef.fields) {
        const el = document.getElementById('f-custom-' + f.name);
        if (el) {
            const val = el.value.trim();
            if (f.required && !val) { toast(f.label + ' is required', 'error'); return null; }
            if (val) data[f.name] = f.field_type === 'number' ? parseFloat(val) : val;
        }
    }
    return { type: 'custom', agent_task_type: typeName, data };
}

function populateTaskForm(task) {
    if (!task) { document.querySelector('input[name="task-type"][value="shell"]').checked = true; updateTaskFields(); return; }
    const type = task.type;
    const radio = document.querySelector('input[name="task-type"][value="' + type + '"]');
    if (radio) radio.checked = true;
    updateTaskFields();
    if (type === 'shell') {
        document.getElementById('f-command').value = task.command || '';
    } else if (type === 'http') {
        document.getElementById('f-http-method').value = task.method || 'get';
        document.getElementById('f-http-url').value = task.url || '';
        document.getElementById('f-http-headers').value = task.headers ? JSON.stringify(task.headers) : '';
        document.getElementById('f-http-body').value = task.body || '';
        document.getElementById('f-http-expect').value = task.expect_status || '';
    } else if (type === 'sql') {
        document.getElementById('f-sql-driver').value = task.driver || 'postgres';
        document.getElementById('f-sql-conn').value = task.connection_string || '';
        document.getElementById('f-sql-query').value = task.query || '';
    } else if (type === 'ftp') {
        document.getElementById('f-ftp-proto').value = task.protocol || 'sftp';
        document.getElementById('f-ftp-dir').value = task.direction || 'download';
        document.getElementById('f-ftp-host').value = task.host || '';
        document.getElementById('f-ftp-port').value = task.port || '';
        document.getElementById('f-ftp-user').value = task.username || '';
        document.getElementById('f-ftp-pass').value = task.password || '';
        document.getElementById('f-ftp-remote').value = task.remote_path || '';
        document.getElementById('f-ftp-local').value = task.local_path || '';
    } else if (type === 'script') {
        populateScriptDropdown(task.script_name);
    } else if (type === 'file_push') {
        document.getElementById('f-filepush-dest').value = task.destination || '';
        document.getElementById('f-filepush-perms').value = task.permissions || '';
        document.getElementById('f-filepush-overwrite').checked = task.overwrite !== false;
        filePushFilename = task.filename || '';
        filePushBase64 = task.content_base64 || '';
        filePushSize = filePushBase64 ? Math.floor(filePushBase64.length * 3 / 4) : 0;
        const info = document.getElementById('f-filepush-info');
        if (filePushFilename) {
            info.textContent = filePushFilename + ' (' + (filePushSize / 1024).toFixed(1) + ' KB) — re-upload to change';
            info.style.display = '';
        }
    } else if (type === 'kafka') {
        document.getElementById('f-kafka-broker').value = task.broker || '';
        document.getElementById('f-kafka-topic').value = task.topic || '';
        document.getElementById('f-kafka-message').value = task.message || '';
        document.getElementById('f-kafka-key').value = task.key || '';
        document.getElementById('f-kafka-props').value = task.properties || '';
    } else if (type === 'rabbitmq') {
        document.getElementById('f-rabbitmq-url').value = task.url || '';
        document.getElementById('f-rabbitmq-exchange').value = task.exchange || '';
        document.getElementById('f-rabbitmq-routing').value = task.routing_key || '';
        document.getElementById('f-rabbitmq-message').value = task.message || '';
        document.getElementById('f-rabbitmq-ctype').value = task.content_type || '';
    } else if (type === 'mqtt') {
        document.getElementById('f-mqtt-broker').value = task.broker || '';
        document.getElementById('f-mqtt-port').value = task.port || '';
        document.getElementById('f-mqtt-topic').value = task.topic || '';
        document.getElementById('f-mqtt-message').value = task.message || '';
        document.getElementById('f-mqtt-qos').value = task.qos != null ? task.qos : 1;
        document.getElementById('f-mqtt-user').value = task.username || '';
        document.getElementById('f-mqtt-pass').value = task.password || '';
        document.getElementById('f-mqtt-clientid').value = task.client_id || '';
    } else if (type === 'redis') {
        document.getElementById('f-redis-url').value = task.url || '';
        document.getElementById('f-redis-channel').value = task.channel || '';
        document.getElementById('f-redis-message').value = task.message || '';
    } else if (type === 'mcp') {
        document.getElementById('f-mcp-server').value = task.server_url || '';
        const toolSelect = document.getElementById('f-mcp-tool');
        toolSelect.innerHTML = '<option value="' + esc(task.tool || '') + '" selected>' + esc(task.tool || '') + '</option>';
        document.getElementById('f-mcp-args').value = task.arguments ? JSON.stringify(task.arguments, null, 2) : '';
    }
}

async function discoverMcpTools() {
    const server = document.getElementById('f-mcp-server').value.trim();
    if (!server) { toast('Enter a server first', 'error'); return; }
    try {
        const data = await api('GET', '/api/mcp/tools?server_url=' + encodeURIComponent(server));
        const toolSelect = document.getElementById('f-mcp-tool');
        toolSelect.innerHTML = '<option value="">Select a tool...</option>';
        for (const t of data.tools) {
            toolSelect.innerHTML += '<option value="' + esc(t.name) + '">' + esc(t.name) + (t.description ? ' — ' + esc(t.description) : '') + '</option>';
        }
        if (data.tools.length === 0) toast('No tools found on this server', 'info');
        else toast(data.tools.length + ' tools discovered');
    } catch (e) {
        toast('Discovery failed: ' + e.message, 'error');
    }
}

// --- Cron Builder ---

function switchCronMode(mode, btn) {
    btn.parentElement.querySelectorAll('.output-tab').forEach(t => t.classList.remove('active'));
    btn.classList.add('active');
    document.getElementById('cron-builder').style.display = mode === 'builder' ? '' : 'none';
    document.getElementById('cron-raw').style.display = mode === 'raw' ? '' : 'none';
    if (mode === 'raw') {
        // Sync builder -> raw
        document.getElementById('f-cron').value = document.getElementById('cb-preview').textContent;
    } else {
        // Try to parse raw -> builder
        parseCronToUI(document.getElementById('f-cron').value.trim());
    }
}

function toggleDow(btn) {
    btn.classList.toggle('active');
    buildCronFromUI();
}

function updateCronOptions() {
    const unit = document.getElementById('cb-unit').value;
    document.getElementById('cb-at-time-group').style.display = (unit === 'day' || unit === 'week' || unit === 'month') ? '' : 'none';
    document.getElementById('cb-dow-group').style.display = unit === 'week' ? '' : 'none';
    document.getElementById('cb-dom-group').style.display = unit === 'month' ? '' : 'none';
}

function buildCronFromUI() {
    const unit = document.getElementById('cb-unit').value;
    const interval = parseInt(document.getElementById('cb-interval').value) || 1;
    const hour = parseInt(document.getElementById('cb-hour').value) || 0;
    const minute = parseInt(document.getElementById('cb-minute').value) || 0;

    let expr = '';
    let desc = '';

    if (unit === 'minute') {
        if (interval === 1) { expr = '0 * * * * *'; desc = 'Every minute'; }
        else { expr = '0 */' + interval + ' * * * *'; desc = 'Every ' + interval + ' minutes'; }
    } else if (unit === 'hour') {
        if (interval === 1) { expr = '0 0 * * * *'; desc = 'Every hour'; }
        else { expr = '0 0 */' + interval + ' * * *'; desc = 'Every ' + interval + ' hours'; }
    } else if (unit === 'day') {
        const pad = n => String(n).padStart(2, '0');
        if (interval === 1) { expr = '0 ' + minute + ' ' + hour + ' * * *'; desc = 'Daily at ' + pad(hour) + ':' + pad(minute); }
        else { expr = '0 ' + minute + ' ' + hour + ' */' + interval + ' * *'; desc = 'Every ' + interval + ' days at ' + pad(hour) + ':' + pad(minute); }
    } else if (unit === 'week') {
        const pad = n => String(n).padStart(2, '0');
        const selectedDow = Array.from(document.querySelectorAll('.cron-dow.active')).map(b => b.dataset.dow);
        const dow = selectedDow.length > 0 ? selectedDow.join(',') : '*';
        const dayNames = {0:'Sun',1:'Mon',2:'Tue',3:'Wed',4:'Thu',5:'Fri',6:'Sat'};
        const dowDesc = selectedDow.length > 0 ? selectedDow.map(d => dayNames[d]).join(', ') : 'every day';
        expr = '0 ' + minute + ' ' + hour + ' * * ' + dow;
        desc = 'Weekly on ' + dowDesc + ' at ' + pad(hour) + ':' + pad(minute);
    } else if (unit === 'month') {
        const pad = n => String(n).padStart(2, '0');
        const dom = parseInt(document.getElementById('cb-dom').value) || 1;
        expr = '0 ' + minute + ' ' + hour + ' ' + dom + ' * *';
        desc = 'Monthly on day ' + dom + ' at ' + pad(hour) + ':' + pad(minute);
    }

    document.getElementById('cb-preview').textContent = expr;
    document.getElementById('cb-description').textContent = desc;
    document.getElementById('f-cron').value = expr;
}

function parseCronToUI(expr) {
    if (!expr) { buildCronFromUI(); return; }
    const parts = expr.split(/\s+/);
    if (parts.length !== 6) { buildCronFromUI(); return; }

    const [sec, min, hr, dom, mon, dow] = parts;

    // Reset
    document.querySelectorAll('.cron-dow').forEach(b => b.classList.remove('active'));

    // Try to detect pattern
    if (min.startsWith('*/')) {
        document.getElementById('cb-unit').value = 'minute';
        document.getElementById('cb-interval').value = parseInt(min.slice(2));
    } else if (hr.startsWith('*/')) {
        document.getElementById('cb-unit').value = 'hour';
        document.getElementById('cb-interval').value = parseInt(hr.slice(2));
    } else if (dow !== '*') {
        document.getElementById('cb-unit').value = 'week';
        document.getElementById('cb-interval').value = 1;
        document.getElementById('cb-hour').value = hr === '*' ? 0 : parseInt(hr);
        document.getElementById('cb-minute').value = min === '*' ? 0 : parseInt(min);
        dow.split(',').forEach(d => {
            const btn = document.querySelector('.cron-dow[data-dow="' + d.trim() + '"]');
            if (btn) btn.classList.add('active');
        });
    } else if (dom !== '*' && !dom.startsWith('*/')) {
        document.getElementById('cb-unit').value = 'month';
        document.getElementById('cb-interval').value = 1;
        document.getElementById('cb-dom').value = parseInt(dom);
        document.getElementById('cb-hour').value = hr === '*' ? 0 : parseInt(hr);
        document.getElementById('cb-minute').value = min === '*' ? 0 : parseInt(min);
    } else if (hr !== '*' && min !== '*') {
        document.getElementById('cb-unit').value = 'day';
        document.getElementById('cb-interval').value = dom.startsWith('*/') ? parseInt(dom.slice(2)) : 1;
        document.getElementById('cb-hour').value = parseInt(hr);
        document.getElementById('cb-minute').value = parseInt(min);
    } else if (min === '*' && hr === '*') {
        document.getElementById('cb-unit').value = 'minute';
        document.getElementById('cb-interval').value = 1;
    } else {
        document.getElementById('cb-unit').value = 'minute';
        document.getElementById('cb-interval').value = 1;
    }

    updateCronOptions();
    buildCronFromUI();
}

function updateCronPreviewFromRaw() {
    // Just sync the raw input — don't update builder to avoid loops
}

function getCronValue() {
    return document.getElementById('f-cron').value.trim() || document.getElementById('cb-preview').textContent;
}

function onEventKindSelect() {
    const select = document.getElementById('f-event-kind-select');
    const custom = document.getElementById('f-event-kind-custom');
    const hidden = document.getElementById('f-event-kind');
    if (select.value === '__custom__') {
        custom.style.display = '';
        custom.focus();
        hidden.value = custom.value || '';
        custom.oninput = () => { hidden.value = custom.value; };
    } else {
        custom.style.display = 'none';
        hidden.value = select.value;
    }
}

function setEventKindValue(val) {
    const select = document.getElementById('f-event-kind-select');
    const custom = document.getElementById('f-event-kind-custom');
    const hidden = document.getElementById('f-event-kind');
    hidden.value = val;
    // Check if val matches a dropdown option
    const option = Array.from(select.options).find(o => o.value === val && o.value !== '__custom__');
    if (option) {
        select.value = val;
        custom.style.display = 'none';
    } else {
        select.value = '__custom__';
        custom.value = val;
        custom.style.display = '';
    }
}

function updateSchedFields() {
    const type = document.querySelector('input[name="sched-type"]:checked').value;
    document.getElementById('cron-field').style.display = type === 'cron' ? '' : 'none';
    document.getElementById('oneshot-field').style.display = type === 'one_shot' ? '' : 'none';
    document.getElementById('event-field').style.display = type === 'event' ? '' : 'none';
}

let currentExecMode = 'standard';
let selectedCustomAgentData = null;

function switchJobTab(tabId, btn) {
    document.querySelectorAll('.modal-tab-content').forEach(el => {
        if (el.id.startsWith('job-tab-')) el.classList.remove('active');
    });
    document.querySelectorAll('.modal-tabs .modal-tab').forEach(t => t.classList.remove('active'));
    document.getElementById('job-tab-' + tabId).classList.add('active');
    if (btn) btn.classList.add('active');
}

function resetJobTabs() {
    document.querySelectorAll('.modal-tabs .modal-tab').forEach((t, i) => {
        t.classList.toggle('active', i === 0);
    });
    document.querySelectorAll('.modal-tab-content').forEach(el => {
        if (el.id.startsWith('job-tab-')) el.classList.toggle('active', el.id === 'job-tab-task');
    });
}

function updateExecutionMode() {
    currentExecMode = document.querySelector('input[name="exec-mode"]:checked').value;
    const isCustom = currentExecMode === 'custom';
    document.getElementById('builtin-task-section').style.display = isCustom ? 'none' : '';
    document.getElementById('custom-agent-section').style.display = isCustom ? '' : 'none';
    document.getElementById('standard-target-section').style.display = isCustom ? 'none' : '';
    if (isCustom) {
        populateCustomAgentSelect();
    }
}

async function populateCustomAgentSelect() {
    const select = document.getElementById('f-custom-agent');
    try {
        const agents = await api('GET', '/api/agents');
        const custom = agents.filter(a => a.status === 'online' && a.agent_type === 'custom');
        if (custom.length === 0) {
            select.innerHTML = '<option value="">No online custom agents</option>';
        } else {
            select.innerHTML = '<option value="">Select a custom agent...</option>' + custom.map(a =>
                '<option value="' + a.id + '">' + esc(a.name) + ' (' + a.hostname + ')' + '</option>'
            ).join('');
        }
        document.getElementById('custom-task-type-group').style.display = 'none';
        document.getElementById('custom-task-fields').innerHTML = '';
        selectedCustomAgentData = null;
    } catch (e) {
        select.innerHTML = '<option value="">Failed to load agents</option>';
    }
}

async function onCustomAgentSelected() {
    const agentId = document.getElementById('f-custom-agent').value;
    const taskTypeGroup = document.getElementById('custom-task-type-group');
    const fieldsDiv = document.getElementById('custom-task-fields');
    if (!agentId) {
        taskTypeGroup.style.display = 'none';
        fieldsDiv.innerHTML = '';
        selectedCustomAgentData = null;
        return;
    }
    // Find agent in cached list or fetch
    try {
        const agents = await api('GET', '/api/agents');
        const agent = agents.find(a => a.id === agentId);
        if (!agent || !agent.task_types || agent.task_types.length === 0) {
            taskTypeGroup.style.display = 'none';
            fieldsDiv.innerHTML = '<div class="form-hint">This agent has no registered task types</div>';
            selectedCustomAgentData = null;
            return;
        }
        selectedCustomAgentData = agent;
        // Render task type radio buttons
        let html = '';
        agent.task_types.forEach((tt, i) => {
            html += '<label><input type="radio" name="custom-task-type" value="' + esc(tt.name) + '"' + (i === 0 ? ' checked' : '') + ' onchange="onCustomTaskTypeSelected()"> ' + esc(tt.name) + (tt.description ? ' <span style="color:var(--text-muted);font-size:11px">(' + esc(tt.description) + ')</span>' : '') + '</label>';
        });
        document.getElementById('custom-task-types').innerHTML = html;
        taskTypeGroup.style.display = '';
        onCustomTaskTypeSelected();
    } catch (e) {
        fieldsDiv.innerHTML = '<div class="form-hint" style="color:var(--danger)">Failed to load agent details</div>';
    }
}

function onCustomTaskTypeSelected() {
    const fieldsDiv = document.getElementById('custom-task-fields');
    if (!selectedCustomAgentData) { fieldsDiv.innerHTML = ''; return; }
    const typeName = document.querySelector('input[name="custom-task-type"]:checked')?.value;
    if (!typeName) { fieldsDiv.innerHTML = ''; return; }
    const taskDef = selectedCustomAgentData.task_types.find(t => t.name === typeName);
    if (!taskDef || !taskDef.fields || taskDef.fields.length === 0) {
        fieldsDiv.innerHTML = '<div class="form-hint">No fields defined for this task type</div>';
        return;
    }
    let html = '';
    for (const f of taskDef.fields) {
        html += formField({
            type: f.field_type || 'text',
            id: 'f-custom-' + f.name,
            label: f.label + (f.required ? ' *' : ''),
            placeholder: f.placeholder || '',
            options: f.options ? f.options.map(o => ({ value: o.value, label: o.label })) : undefined
        });
    }
    fieldsDiv.innerHTML = html;
}

function updateTargetFields() {
    const type = document.querySelector('input[name="target-type"]:checked').value;
    document.getElementById('agent-select-field').style.display = type === 'agent' ? '' : 'none';
    if (type === 'agent') {
        populateAgentSelect('standard');
    }
}

async function populateAgentSelect(filterType) {
    const select = document.getElementById('f-agent');
    try {
        const agents = await api('GET', '/api/agents');
        let online = agents.filter(a => a.status === 'online');
        if (filterType) {
            online = online.filter(a => (a.agent_type || 'standard') === filterType);
        }
        if (online.length === 0) {
            select.innerHTML = '<option value="">No online ' + (filterType || '') + ' agents</option>';
        } else {
            select.innerHTML = online.map(a => {
                const typeLabel = a.agent_type === 'custom' ? ' \u2022 custom' : '';
                return '<option value="' + a.id + '" data-agent-type="' + (a.agent_type || 'standard') + '">' + esc(a.name) + ' (' + a.hostname + typeLabel + ')' + (a.tags.length ? ' [' + a.tags.join(', ') + ']' : '') + '</option>';
            }).join('');
        }
        select.onchange = updateAgentHint;
        updateAgentHint();
    } catch (e) {
        select.innerHTML = '<option value="">Failed to load agents</option>';
    }
}

function updateAgentHint() {
    const select = document.getElementById('f-agent');
    let hint = document.getElementById('agent-type-hint');
    if (!hint) {
        hint = document.createElement('div');
        hint.id = 'agent-type-hint';
        hint.className = 'form-hint';
        select.parentElement.appendChild(hint);
    }
    const selected = select.options[select.selectedIndex];
    if (selected && selected.dataset.agentType === 'custom') {
        hint.innerHTML = 'Custom agent \u2014 job will be <strong>queued</strong> until the agent polls for work';
        hint.style.display = '';
    } else {
        hint.style.display = 'none';
    }
}

function populateDeps(excludeId, selected) {
    const container = document.getElementById('deps-entries');
    container.innerHTML = '';
    const sel = selected || [];
    for (const dep of sel) {
        addDepEntry(dep.job_id, dep.within_secs, excludeId);
    }
    updateDepsEmpty();
}

function addDepEntry(jobId, withinSecs, excludeId) {
    const container = document.getElementById('deps-entries');
    const exclude = excludeId || editingJobId;
    const jobs = allJobs.filter(j => j.id !== exclude);
    if (jobs.length === 0) {
        toast('No other jobs available to depend on', 'error');
        return;
    }

    // Convert seconds to best unit for display
    let windowVal = '';
    let windowUnit = '60'; // default to minutes
    if (withinSecs) {
        if (withinSecs % 86400 === 0) { windowVal = withinSecs / 86400; windowUnit = '86400'; }
        else if (withinSecs % 3600 === 0) { windowVal = withinSecs / 3600; windowUnit = '3600'; }
        else if (withinSecs % 60 === 0) { windowVal = withinSecs / 60; windowUnit = '60'; }
        else { windowVal = withinSecs; windowUnit = '1'; }
    }

    const div = document.createElement('div');
    div.className = 'dep-entry';

    let html = '<select class="dep-job-select"><option value="">Select job...</option>';
    for (const j of jobs) {
        const sel = j.id === jobId ? ' selected' : '';
        html += '<option value="' + j.id + '"' + sel + '>' + esc(j.name) + '</option>';
    }
    html += '</select>';
    html += '<div class="dep-window-group">';
    html += '<span class="dep-window-label">succeeded within</span>';
    html += '<input type="number" class="dep-window-val" min="1" placeholder="\u221E" value="' + windowVal + '" title="Time window (empty = no limit)">';
    html += '<select class="dep-window-unit">';
    html += '<option value="1"' + (windowUnit === '1' ? ' selected' : '') + '>sec</option>';
    html += '<option value="60"' + (windowUnit === '60' ? ' selected' : '') + '>min</option>';
    html += '<option value="3600"' + (windowUnit === '3600' ? ' selected' : '') + '>hr</option>';
    html += '<option value="86400"' + (windowUnit === '86400' ? ' selected' : '') + '>day</option>';
    html += '</select>';
    html += '</div>';
    html += '<button type="button" class="dep-remove" onclick="removeDepEntry(this)" title="Remove">&times;</button>';

    div.innerHTML = html;
    container.appendChild(div);
    updateDepsEmpty();
}

function removeDepEntry(btn) {
    btn.closest('.dep-entry').remove();
    updateDepsEmpty();
}

function updateDepsEmpty() {
    const container = document.getElementById('deps-entries');
    const existing = container.querySelectorAll('.dep-entry');
    const emptyMsg = container.querySelector('.no-deps-text');
    if (existing.length === 0 && !emptyMsg) {
        const p = document.createElement('div');
        p.className = 'no-deps-text';
        p.textContent = 'No dependencies configured';
        container.appendChild(p);
    } else if (existing.length > 0 && emptyMsg) {
        emptyMsg.remove();
    }
}

function getDepEntries() {
    const entries = document.querySelectorAll('#deps-entries .dep-entry');
    const deps = [];
    for (const entry of entries) {
        const jobId = entry.querySelector('.dep-job-select').value;
        const val = entry.querySelector('.dep-window-val').value.trim();
        const unit = parseInt(entry.querySelector('.dep-window-unit').value);
        if (!jobId) continue;
        deps.push({
            job_id: jobId,
            within_secs: val ? parseInt(val) * unit : null,
        });
    }
    return deps;
}

function addExtractionRow(name, pattern, type, writeToVar, target) {
    const container = document.getElementById('extractions-container');
    const t = target || 'variable';
    const row = document.createElement('div');
    row.className = 'tt-field-row';
    row.innerHTML =
        '<input type="text" value="' + esc(name || '') + '" placeholder="name" style="width:80px" class="ex-name">' +
        '<input type="text" value="' + esc(pattern || '') + '" placeholder="pattern (regex or $.path)" style="flex:1;min-width:120px" class="ex-pattern">' +
        '<select class="ex-type" style="width:90px"><option value="regex"' + (type === 'jsonpath' ? '' : ' selected') + '>regex</option><option value="jsonpath"' + (type === 'jsonpath' ? ' selected' : '') + '>jsonpath</option></select>' +
        '<select class="ex-target" style="width:90px" title="Where to store extracted value"><option value="variable"' + (t === 'output' ? '' : ' selected') + '>Variable</option><option value="output"' + (t === 'output' ? ' selected' : '') + '>Output</option></select>' +
        '<input type="text" value="' + esc(writeToVar || '') + '" placeholder="write to var" title="Write to global variable (variable target only)" style="width:100px" class="ex-write-var">' +
        '<button class="btn btn-ghost btn-sm" style="color:var(--danger);padding:2px 6px" onclick="this.parentElement.remove()">&times;</button>';
    container.appendChild(row);
    // Show/hide write-var field based on target
    const targetSel = row.querySelector('.ex-target');
    const writeVar = row.querySelector('.ex-write-var');
    function toggleWriteVar() { writeVar.style.display = targetSel.value === 'variable' ? '' : 'none'; }
    targetSel.addEventListener('change', toggleWriteVar);
    toggleWriteVar();
}

function addTriggerRow(pattern, severity) {
    const container = document.getElementById('triggers-container');
    const row = document.createElement('div');
    row.className = 'tt-field-row';
    const sev = severity || 'error';
    row.innerHTML =
        '<input type="text" value="' + esc(pattern || '') + '" placeholder="pattern (regex or substring)" style="flex:1;min-width:150px" class="trig-pattern">' +
        '<select class="trig-severity" style="width:90px">' +
        '<option value="error"' + (sev === 'error' ? ' selected' : '') + '>error</option>' +
        '<option value="warning"' + (sev === 'warning' ? ' selected' : '') + '>warning</option>' +
        '<option value="info"' + (sev === 'info' ? ' selected' : '') + '>info</option>' +
        '<option value="success"' + (sev === 'success' ? ' selected' : '') + '>success</option></select>' +
        '<button class="btn btn-ghost btn-sm" style="color:var(--danger);padding:2px 6px" onclick="this.parentElement.remove()">&times;</button>';
    container.appendChild(row);
}

function addAssertionRow(pattern, message) {
    const container = document.getElementById('assertions-container');
    const row = document.createElement('div');
    row.className = 'tt-field-row';
    row.innerHTML =
        '<input type="text" value="' + esc(pattern || '') + '" placeholder="pattern that MUST appear in output" style="flex:1;min-width:150px" class="assert-pattern">' +
        '<input type="text" value="' + esc(message || '') + '" placeholder="failure message (optional)" style="flex:1;min-width:120px" class="assert-message">' +
        '<button class="btn btn-ghost btn-sm" style="color:var(--danger);padding:2px 6px" onclick="this.parentElement.remove()">&times;</button>';
    container.appendChild(row);
}

function collectOutputRules() {
    const extractions = [];
    document.querySelectorAll('#extractions-container .tt-field-row').forEach(row => {
        const name = row.querySelector('.ex-name').value.trim();
        const pattern = row.querySelector('.ex-pattern').value.trim();
        const type = row.querySelector('.ex-type').value;
        const target = row.querySelector('.ex-target').value || 'variable';
        const write_to_variable = row.querySelector('.ex-write-var').value.trim() || null;
        if (name && pattern) {
            const rule = { name, pattern, type, target };
            if (target === 'variable' && write_to_variable) rule.write_to_variable = write_to_variable;
            extractions.push(rule);
        }
    });
    const triggers = [];
    document.querySelectorAll('#triggers-container .tt-field-row').forEach(row => {
        const pattern = row.querySelector('.trig-pattern').value.trim();
        const severity = row.querySelector('.trig-severity').value;
        if (pattern) triggers.push({ pattern, severity });
    });
    const assertions = [];
    document.querySelectorAll('#assertions-container .tt-field-row').forEach(row => {
        const pattern = row.querySelector('.assert-pattern').value.trim();
        const message = row.querySelector('.assert-message').value.trim();
        if (pattern) assertions.push({ pattern, message: message || null });
    });
    if (extractions.length === 0 && triggers.length === 0 && assertions.length === 0) return null;
    return { extractions, triggers, assertions };
}

function populateOutputRules(rules) {
    document.getElementById('extractions-container').innerHTML = '';
    document.getElementById('triggers-container').innerHTML = '';
    document.getElementById('assertions-container').innerHTML = '';
    if (!rules) return;
    (rules.extractions || []).forEach(r => addExtractionRow(r.name, r.pattern, r.type, r.write_to_variable, r.target));
    (rules.triggers || []).forEach(t => addTriggerRow(t.pattern, t.severity));
    (rules.assertions || []).forEach(a => addAssertionRow(a.pattern, a.message));
}

async function submitJobForm() {
    const name = document.getElementById('f-name').value.trim();
    if (!name) { toast('Name is required', 'error'); return; }
    let task;
    if (currentExecMode === 'custom') {
        task = buildCustomTaskFromForm();
    } else {
        task = buildTaskFromForm();
    }
    if (!task) { toast('Task configuration is incomplete', 'error'); return; }

    const schedType = document.querySelector('input[name="sched-type"]:checked').value;
    let schedule;
    if (schedType === 'cron') {
        const expr = getCronValue();
        if (!expr) { toast('Cron expression is required', 'error'); return; }
        schedule = { type: 'cron', value: expr };
    } else if (schedType === 'one_shot') {
        const dt = document.getElementById('f-oneshot').value;
        if (!dt) { toast('Date/time is required', 'error'); return; }
        schedule = { type: 'one_shot', value: new Date(dt).toISOString() };
    } else if (schedType === 'event') {
        const kindPattern = document.getElementById('f-event-kind').value;
        const config = { kind_pattern: kindPattern };
        const sev = document.getElementById('f-event-severity').value;
        if (sev) config.severity = sev;
        const jobFilter = document.getElementById('f-event-job-filter').value.trim();
        if (jobFilter) config.job_name_filter = jobFilter;
        schedule = { type: 'event', value: config };
    } else {
        schedule = { type: 'on_demand' };
    }

    const timeoutVal = document.getElementById('f-timeout').value;
    const timeout_secs = timeoutVal ? parseInt(timeoutVal) : null;

    const depends_on = getDepEntries();

    // Build target
    let target = null;
    if (currentExecMode === 'custom') {
        const customAgentId = document.getElementById('f-custom-agent').value;
        if (!customAgentId) { toast('Select a custom agent', 'error'); return; }
        target = { type: 'agent', agent_id: customAgentId };
    } else {
        const targetType = document.querySelector('input[name="target-type"]:checked').value;
        if (targetType === 'agent') {
            const agentId = document.getElementById('f-agent').value;
            if (!agentId) { toast('Select an agent', 'error'); return; }
            target = { type: 'agent', agent_id: agentId };
        } else if (targetType === 'any') {
            target = { type: 'any' };
        } else if (targetType === 'all') {
            target = { type: 'all' };
        }
    }

    const output_rules = collectOutputRules();
    const notifications = collectJobNotifications();
    const body = { name, task, schedule, timeout_secs, depends_on, target, output_rules, notifications };
    const run_as = document.getElementById('f-run-as').value.trim();
    if (run_as) body.run_as = run_as;
    const desc = document.getElementById('f-desc').value.trim();
    if (desc) body.description = desc;
    const group = document.getElementById('f-group').value.trim();
    body.group = group || null;
    const retryMax = parseInt(document.getElementById('f-retry-max').value) || 0;
    if (retryMax > 0) body.retry_max = retryMax;
    const retryDelay = parseInt(document.getElementById('f-retry-delay').value) || 0;
    if (retryDelay > 0) body.retry_delay_secs = retryDelay;
    const retryBackoff = parseFloat(document.getElementById('f-retry-backoff').value) || 1.0;
    if (retryBackoff !== 1.0) body.retry_backoff = retryBackoff;

    try {
        if (editingJobId) {
            await api('PUT', '/api/jobs/' + editingJobId, body);
            toast('Job updated');
        } else {
            await api('POST', '/api/jobs', body);
            toast('Job created');
        }
        closeCreateModal();
        if (currentJobId) {
            showJobDetail(currentJobId);
        } else if (currentPage !== 'jobs') {
            showPage('jobs');
        } else {
            fetchJobs();
        }
    } catch (e) {
        toast(e.message, 'error');
    }
}

// --- Dependency Map ---

async function renderMap() {
    // Fetch all jobs (unpaginated) for the map
    let jobs;
    try {
        const res = await api('GET', '/api/jobs?per_page=100');
        jobs = res.data;
    } catch (e) {
        console.error('renderMap:', e);
        return;
    }

    const container = document.getElementById('map-container');

    // Populate group filter dropdown
    const mapGroupFilter = document.getElementById('map-group-filter');
    if (mapGroupFilter) {
        const selectedGroup = mapGroupFilter.value;
        const groups = [...new Set(jobs.map(j => j.group || 'Default'))].sort();
        mapGroupFilter.innerHTML = '<option value="">All Groups</option>';
        for (const g of groups) {
            mapGroupFilter.innerHTML += '<option value="' + esc(g) + '"' + (g === selectedGroup ? ' selected' : '') + '>' + esc(g) + '</option>';
        }
        // Filter jobs by selected group
        if (selectedGroup) {
            jobs = jobs.filter(j => (j.group || 'Default') === selectedGroup);
        }
    }

    if (jobs.length === 0) {
        container.innerHTML = renderRichEmptyState({
            icon: '&#9741;',
            title: 'No jobs to display',
            description: 'The dependency map visualizes how jobs depend on each other. Create jobs with dependencies to see the graph.',
            actions: [
                { label: 'Create a Job', onclick: 'openCreateModal()', primary: true },
            ],
        });
        return;
    }

    // Restore the SVG element if it was replaced by empty state
    if (!document.getElementById('map-svg')) {
        container.innerHTML = '<svg id="map-svg"></svg>';
    }
    const svg = document.getElementById('map-svg');

    // Build adjacency: job -> jobs that depend on it (children)
    const jobMap = {};
    for (const j of jobs) jobMap[j.id] = j;

    const children = {};   // parent_id -> [child_id]
    const parents = {};    // child_id -> [parent_id]
    for (const j of jobs) {
        children[j.id] = children[j.id] || [];
        parents[j.id] = parents[j.id] || [];
        for (const dep of j.depends_on) {
            const pid = dep.job_id;
            children[pid] = children[pid] || [];
            children[pid].push(j.id);
            parents[j.id].push(pid);
        }
    }

    // Include event trigger relationships in the graph for layout
    for (const j of jobs) {
        if (j.schedule.type !== 'event' || !j.schedule.value || !j.schedule.value.job_name_filter) continue;
        const filter = j.schedule.value.job_name_filter.toLowerCase();
        for (const src of jobs) {
            if (src.id === j.id) continue;
            if (src.name.toLowerCase().includes(filter)) {
                children[src.id] = children[src.id] || [];
                if (!children[src.id].includes(j.id)) children[src.id].push(j.id);
                parents[j.id] = parents[j.id] || [];
                if (!parents[j.id].includes(src.id)) parents[j.id].push(src.id);
            }
        }
    }

    // Assign layers via BFS from roots (nodes with no parents in the graph)
    const layers = {};
    const roots = jobs.filter(j => (parents[j.id] || []).length === 0).map(j => j.id);
    const visited = new Set();
    const queue = roots.map(id => ({ id, layer: 0 }));
    // Also handle jobs whose parents aren't in the job list
    for (const j of jobs) {
        if (!roots.includes(j.id) && (parents[j.id] || []).every(pid => !jobMap[pid])) {
            queue.push({ id: j.id, layer: 0 });
        }
    }

    while (queue.length > 0) {
        const { id, layer } = queue.shift();
        if (visited.has(id)) {
            layers[id] = Math.max(layers[id] || 0, layer);
            continue;
        }
        visited.add(id);
        layers[id] = layer;
        for (const cid of (children[id] || [])) {
            queue.push({ id: cid, layer: layer + 1 });
        }
    }

    // Handle any unvisited (disconnected) jobs
    for (const j of jobs) {
        if (!visited.has(j.id)) {
            layers[j.id] = 0;
        }
    }

    // Group by layer
    const layerGroups = {};
    let maxLayer = 0;
    for (const [id, layer] of Object.entries(layers)) {
        layerGroups[layer] = layerGroups[layer] || [];
        layerGroups[layer].push(id);
        maxLayer = Math.max(maxLayer, layer);
    }

    // Layout constants
    const nodeW = 180;
    const nodeH = 56;
    const layerGap = 100;
    const nodeGap = 20;
    const padX = 40;
    const padY = 40;

    // Position nodes
    const positions = {};
    let totalW = 0;
    let totalH = 0;

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

    let svgHtml = '';

    // Defs for arrowhead
    svgHtml += '<defs><marker id="arrow" viewBox="0 0 10 6" refX="10" refY="3" markerWidth="8" markerHeight="6" orient="auto-start-reverse"><path d="M 0 0 L 10 3 L 0 6 z" class="map-arrowhead"/></marker></defs>';

    // Draw edges
    for (const j of jobs) {
        for (const dep of j.depends_on) {
            const from = positions[dep.job_id];
            const to = positions[j.id];
            if (!from || !to) continue;

            const x1 = from.x + nodeW;
            const y1 = from.y + nodeH / 2;
            const x2 = to.x;
            const y2 = to.y + nodeH / 2;
            const cx1 = x1 + (x2 - x1) * 0.4;
            const cx2 = x2 - (x2 - x1) * 0.4;

            // Window label
            let label = '';
            if (dep.within_secs) {
                const mid_x = (x1 + x2) / 2;
                const mid_y = (y1 + y2) / 2 - 8;
                label = '<text x="' + mid_x + '" y="' + mid_y + '" text-anchor="middle" font-size="9" fill="' + 'var(--text-muted)' + '">within ' + fmtSeconds(dep.within_secs) + '</text>';
            }

            svgHtml += '<path d="M ' + x1 + ' ' + y1 + ' C ' + cx1 + ' ' + y1 + ', ' + cx2 + ' ' + y2 + ', ' + x2 + ' ' + y2 + '" class="map-edge" stroke="var(--text-muted)" marker-end="url(#arrow)"/>' + label;
        }
    }

    // Draw event trigger edges (dashed) — only when a specific job_name_filter is set
    for (const j of jobs) {
        if (j.schedule.type !== 'event' || !j.schedule.value) continue;
        const to = positions[j.id];
        if (!to) continue;
        const filter = j.schedule.value.job_name_filter;
        const kindPattern = j.schedule.value.kind_pattern || '*';
        // Only draw specific edges when a job name filter identifies the source
        if (!filter) continue;
        for (const src of jobs) {
            if (src.id === j.id) continue;
            if (!src.name.toLowerCase().includes(filter.toLowerCase())) continue;
            const from = positions[src.id];
            if (!from) continue;

            // Route the curve: handle same-column or right-to-left cases
            let x1, y1, x2, y2, cx1, cx2;
            if (from.x + nodeW <= to.x) {
                // Normal left-to-right
                x1 = from.x + nodeW; y1 = from.y + nodeH / 2;
                x2 = to.x; y2 = to.y + nodeH / 2;
                cx1 = x1 + (x2 - x1) * 0.4; cx2 = x2 - (x2 - x1) * 0.4;
            } else if (to.x + nodeW <= from.x) {
                // Right-to-left
                x1 = from.x; y1 = from.y + nodeH / 2;
                x2 = to.x + nodeW; y2 = to.y + nodeH / 2;
                cx1 = x1 - Math.abs(x1 - x2) * 0.4; cx2 = x2 + Math.abs(x1 - x2) * 0.4;
            } else {
                // Same column — route below the nodes with a U-shaped curve
                x1 = from.x + nodeW / 2; y1 = from.y + nodeH;
                x2 = to.x + nodeW / 2; y2 = to.y + nodeH;
                const drop = 40;
                cx1 = x1; cx2 = x2;
                // Use a quadratic-ish path going down and across
                svgHtml += '<path d="M ' + x1 + ' ' + y1 + ' C ' + x1 + ' ' + (y1 + drop) + ', ' + x2 + ' ' + (y2 + drop) + ', ' + x2 + ' ' + y2 + '" class="map-edge" stroke="var(--warning)" stroke-dasharray="6 3" fill="none" marker-end="url(#arrow)"/>';
                svgHtml += '<text x="' + ((x1+x2)/2) + '" y="' + (Math.max(y1,y2) + drop - 4) + '" text-anchor="middle" font-size="8" fill="var(--warning)">\u26A1 ' + esc(kindPattern) + '</text>';
                continue;
            }
            svgHtml += '<path d="M ' + x1 + ' ' + y1 + ' C ' + cx1 + ' ' + y1 + ', ' + cx2 + ' ' + y2 + ', ' + x2 + ' ' + y2 + '" class="map-edge" stroke="var(--warning)" stroke-dasharray="6 3" fill="none" marker-end="url(#arrow)"/>';
            svgHtml += '<text x="' + ((x1+x2)/2) + '" y="' + ((y1+y2)/2 - 8) + '" text-anchor="middle" font-size="8" fill="var(--warning)">\u26A1 ' + esc(kindPattern) + '</text>';
        }
    }

    // Draw nodes
    for (const j of jobs) {
        const pos = positions[j.id];
        if (!pos) continue;

        let fill, stroke;
        const lastStatus = j.last_execution ? j.last_execution.status : null;
        if (lastStatus === 'succeeded') { fill = 'rgba(46,204,113,0.15)'; stroke = 'var(--success)'; }
        else if (lastStatus === 'failed' || lastStatus === 'timed_out') { fill = 'rgba(224,82,82,0.15)'; stroke = 'var(--danger)'; }
        else if (lastStatus === 'running') { fill = 'rgba(62,139,255,0.15)'; stroke = 'var(--info)'; }
        else { fill = 'var(--bg-tertiary)'; stroke = 'var(--border)'; }

        if (j.status === 'paused') {
            fill = 'var(--bg-tertiary)'; stroke = 'var(--text-muted)';
        }

        const targetText = j.target ? (j.target.type === 'local' ? '' : j.target.type) : '';

        // Status indicator color
        let dotColor;
        if (lastStatus === 'succeeded') dotColor = 'var(--success)';
        else if (lastStatus === 'failed' || lastStatus === 'timed_out') dotColor = 'var(--danger)';
        else if (lastStatus === 'running') dotColor = 'var(--info)';
        else dotColor = 'var(--text-muted)';

        // Schedule icon
        let schedIcon = '\u23F0'; // alarm clock - cron
        if (j.schedule.type === 'on_demand') schedIcon = '\u270B'; // hand - on-demand
        else if (j.schedule.type === 'event') schedIcon = '\u26A1'; // lightning - event
        else if (j.schedule.type === 'one_shot') schedIcon = '\u26A1'; // lightning - one-shot

        // Target icon
        let targetIcon = '';
        if (j.target && j.target.type !== 'local') targetIcon = ' \uD83D\uDDA5'; // computer

        svgHtml += '<g class="map-node" onclick="showJobDetail(\'' + j.id + '\')">';
        svgHtml += '<rect x="' + pos.x + '" y="' + pos.y + '" width="' + nodeW + '" height="' + nodeH + '" fill="' + fill + '" stroke="' + stroke + '"/>';
        // Status dot
        svgHtml += '<circle cx="' + (pos.x + 14) + '" cy="' + (pos.y + 17) + '" r="4" fill="' + dotColor + '"/>';
        // Name with schedule icon
        svgHtml += '<text class="node-name" x="' + (pos.x + 24) + '" y="' + (pos.y + 20) + '">' + schedIcon + ' ' + esc(j.name) + '</text>';
        // Status line
        svgHtml += '<text class="node-status" x="' + (pos.x + 24) + '" y="' + (pos.y + 35) + '">' + j.status + (lastStatus ? ' \u2022 ' + lastStatus : '') + '</text>';
        // Group badge
        const groupName = j.group || 'Default';
        const gColor = groupColor(groupName);
        svgHtml += '<rect x="' + (pos.x + 24) + '" y="' + (pos.y + 40) + '" width="' + Math.min(groupName.length * 6 + 10, nodeW - 30) + '" height="14" rx="7" fill="' + gColor + '" opacity="0.8"/>';
        svgHtml += '<text x="' + (pos.x + 29) + '" y="' + (pos.y + 50) + '" font-size="8" font-weight="600" fill="#fff">' + esc(groupName) + '</text>';
        // Target
        if (targetText) {
            svgHtml += '<text class="node-target" x="' + (pos.x + nodeW - 8) + '" y="' + (pos.y + 50) + '" text-anchor="end" font-size="8" fill="var(--text-muted)">' + targetIcon + ' ' + targetText + '</text>';
        }
        svgHtml += '</g>';
    }

    svg.innerHTML = svgHtml;
}

// --- Mini Dependency Map ---

function renderMiniMap(job) {
    const card = document.getElementById('mini-map-card');
    const svg = document.getElementById('mini-map-svg');

    // Collect related jobs: this job + its dependencies + jobs that depend on it
    const relatedIds = new Set();
    relatedIds.add(job.id);
    for (const dep of job.depends_on) {
        relatedIds.add(dep.job_id);
    }
    // Find jobs that depend on this job
    for (const j of allJobs) {
        for (const dep of j.depends_on) {
            if (dep.job_id === job.id) {
                relatedIds.add(j.id);
            }
        }
    }

    if (relatedIds.size <= 1 && job.depends_on.length === 0) {
        // No dependencies at all
        card.style.display = 'none';
        return;
    }

    card.style.display = '';

    // Build job lookup from allJobs + current job
    const jobMap = {};
    for (const j of allJobs) jobMap[j.id] = j;
    jobMap[job.id] = job;

    const related = Array.from(relatedIds).map(id => jobMap[id]).filter(Boolean);

    // Build adjacency for layout
    const children = {};
    const parents = {};
    for (const j of related) {
        children[j.id] = children[j.id] || [];
        parents[j.id] = parents[j.id] || [];
        for (const dep of j.depends_on) {
            if (relatedIds.has(dep.job_id)) {
                children[dep.job_id] = children[dep.job_id] || [];
                children[dep.job_id].push(j.id);
                parents[j.id].push(dep.job_id);
            }
        }
    }

    // Layer assignment
    const layers = {};
    const roots = related.filter(j => (parents[j.id] || []).length === 0).map(j => j.id);
    const visited = new Set();
    const queue = roots.map(id => ({ id, layer: 0 }));

    while (queue.length > 0) {
        const { id, layer } = queue.shift();
        if (visited.has(id)) {
            layers[id] = Math.max(layers[id] || 0, layer);
            continue;
        }
        visited.add(id);
        layers[id] = layer;
        for (const cid of (children[id] || [])) {
            queue.push({ id: cid, layer: layer + 1 });
        }
    }
    for (const j of related) {
        if (!visited.has(j.id)) layers[j.id] = 0;
    }

    // Layout
    const nodeW = 150;
    const nodeH = 44;
    const layerGap = 80;
    const nodeGap = 16;
    const padX = 20;
    const padY = 20;

    const layerGroups = {};
    let maxLayer = 0;
    for (const [id, layer] of Object.entries(layers)) {
        layerGroups[layer] = layerGroups[layer] || [];
        layerGroups[layer].push(id);
        maxLayer = Math.max(maxLayer, layer);
    }

    const positions = {};
    let totalW = 0;
    let totalH = 0;
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

    let svgHtml = '<defs><marker id="mini-arrow" viewBox="0 0 10 6" refX="10" refY="3" markerWidth="8" markerHeight="6" orient="auto-start-reverse"><path d="M 0 0 L 10 3 L 0 6 z" class="map-arrowhead"/></marker></defs>';

    // Edges
    for (const j of related) {
        for (const dep of j.depends_on) {
            const from = positions[dep.job_id];
            const to = positions[j.id];
            if (!from || !to) continue;
            const x1 = from.x + nodeW;
            const y1 = from.y + nodeH / 2;
            const x2 = to.x;
            const y2 = to.y + nodeH / 2;
            const cx1 = x1 + (x2 - x1) * 0.4;
            const cx2 = x2 - (x2 - x1) * 0.4;

            let label = '';
            if (dep.within_secs) {
                const mx = (x1 + x2) / 2;
                const my = (y1 + y2) / 2 - 8;
                label = '<text x="' + mx + '" y="' + my + '" text-anchor="middle" font-size="9" fill="var(--text-muted)">within ' + fmtSeconds(dep.within_secs) + '</text>';
            }
            svgHtml += '<path d="M ' + x1 + ' ' + y1 + ' C ' + cx1 + ' ' + y1 + ', ' + cx2 + ' ' + y2 + ', ' + x2 + ' ' + y2 + '" class="map-edge" stroke="var(--text-muted)" marker-end="url(#mini-arrow)"/>' + label;
        }
    }

    // Nodes
    for (const j of related) {
        const pos = positions[j.id];
        if (!pos) continue;

        const isCurrent = j.id === job.id;
        let fill, stroke;
        const lastStatus = j.last_execution ? j.last_execution.status : null;
        if (lastStatus === 'succeeded') { fill = 'rgba(46,204,113,0.15)'; stroke = 'var(--success)'; }
        else if (lastStatus === 'failed' || lastStatus === 'timed_out') { fill = 'rgba(224,82,82,0.15)'; stroke = 'var(--danger)'; }
        else if (lastStatus === 'running') { fill = 'rgba(62,139,255,0.15)'; stroke = 'var(--info)'; }
        else { fill = 'var(--bg-tertiary)'; stroke = 'var(--border)'; }

        if (isCurrent) stroke = 'var(--accent)';

        const cls = isCurrent ? 'map-node mini-map-current' : 'map-node';
        const onclick = isCurrent ? '' : ' onclick="showJobDetail(\'' + j.id + '\')"';

        svgHtml += '<g class="' + cls + '"' + onclick + '>';
        svgHtml += '<rect x="' + pos.x + '" y="' + pos.y + '" width="' + nodeW + '" height="' + nodeH + '" fill="' + fill + '" stroke="' + stroke + '"/>';
        svgHtml += '<circle cx="' + (pos.x + 12) + '" cy="' + (pos.y + 16) + '" r="3" fill="' + (isCurrent ? 'var(--accent)' : 'var(--text-muted)') + '"/>';
        svgHtml += '<text class="node-name" x="' + (pos.x + 20) + '" y="' + (pos.y + 18) + '">' + (isCurrent ? '\u25C9 ' : '') + esc(j.name) + '</text>';
        svgHtml += '<text class="node-status" x="' + (pos.x + 20) + '" y="' + (pos.y + 32) + '">' + j.status + (lastStatus ? ' \u2022 ' + lastStatus : '') + '</text>';
        svgHtml += '</g>';
    }

    svg.innerHTML = svgHtml;
}

function collectJobNotifications() {
    const onFailure = document.getElementById('f-notif-failure').checked;
    const onSuccess = document.getElementById('f-notif-success').checked;
    const onAssertion = document.getElementById('f-notif-assertion').checked;
    if (!onFailure && !onSuccess && !onAssertion) return null;
    const emailsStr = document.getElementById('f-notif-emails').value.trim();
    const config = { on_failure: onFailure, on_success: onSuccess, on_assertion_failure: onAssertion };
    if (emailsStr) {
        config.recipients = { emails: emailsStr.split(',').map(s => s.trim()).filter(Boolean), phones: [] };
    }
    return config;
}

function populateJobNotifications(notif) {
    document.getElementById('f-notif-failure').checked = notif ? notif.on_failure : false;
    document.getElementById('f-notif-success').checked = notif ? notif.on_success : false;
    document.getElementById('f-notif-assertion').checked = notif ? notif.on_assertion_failure : false;
    const emails = notif && notif.recipients ? (notif.recipients.emails || []).join(', ') : '';
    document.getElementById('f-notif-emails').value = emails;
}

function showCreateKeyForm() {
    document.getElementById('create-key-form').style.display = '';
    document.getElementById('new-key-name').focus();
}

function hideCreateKeyForm() {
    document.getElementById('create-key-form').style.display = 'none';
    document.getElementById('new-key-display').style.display = 'none';
}

async function createKey() {
    const name = document.getElementById('new-key-name').value.trim();
    const role = document.getElementById('new-key-role').value;
    if (!name) { toast('Key name is required', 'error'); return; }
    try {
        const res = await api('POST', '/api/keys', { name, role });
        document.getElementById('new-key-display').style.display = '';
        const rawKey = res.raw_key;
        document.getElementById('new-key-display').innerHTML =
            '<strong>Key created!</strong> Copy it now — it won\'t be shown again.' +
            '<code id="new-key-value">' + esc(rawKey) + '</code>' +
            '<button class="btn btn-ghost btn-sm" onclick="copyKey()" style="margin-top:4px">&#128203; Copy to Clipboard</button>';
        document.getElementById('new-key-name').value = '';
        fetchKeys();
    } catch (e) {
        toast(e.message, 'error');
    }
}

async function fetchKeys() {
    try {
        const keys = await api('GET', '/api/keys');
        renderKeys(keys);
    } catch (e) {
        console.error('fetchKeys:', e);
    }
}

function renderKeys(keys) {
    const list = document.getElementById('keys-list');
    if (keys.length === 0) {
        list.innerHTML = '<div style="font-size:13px;color:var(--text-muted)">No API keys</div>';
        return;
    }
    let html = '';
    for (const k of keys) {
        const status = k.active ? '' : ' <span class="badge badge-disabled">revoked</span>';
        html += '<div class="key-row">';
        html += '<div class="key-info">';
        html += '<span>' + esc(k.name) + status + '</span>';
        html += '<span class="key-prefix">' + esc(k.key_prefix) + '...</span>';
        html += badge(k.role);
        html += '<span class="time-text">' + (k.last_used_at ? 'used ' + fmtDate(k.last_used_at) : 'never used') + '</span>';
        html += '</div>';
        if (k.active) {
            html += '<button class="btn btn-danger btn-sm" onclick="revokeKey(\'' + k.id + '\',\'' + esc(k.name) + '\')">Revoke</button>';
        }
        html += '</div>';
    }
    list.innerHTML = html;
}

function getControllerUrl() {
    // Normalize 127.0.0.1 to localhost for cleaner commands
    let origin = window.location.origin;
    if (origin.includes('127.0.0.1')) origin = origin.replace('127.0.0.1', 'localhost');
    return origin;
}

async function updatePairCommand() {
    const el = document.getElementById('pair-cmd-text');
    if (!el) return;
    const host = getControllerUrl();
    try {
        const keys = await api('GET', '/api/keys');
        const agentKey = keys.find(k => k.role === 'agent' && k.active);
        if (agentKey) {
            const cmd = 'KRONFORCE_AGENT_KEY=<agent_key> KRONFORCE_CONTROLLER_URL=' + host + ' cargo run --bin kronforce-agent';
            el.textContent = cmd;
            el.dataset.fullCmd = cmd;
            el.insertAdjacentHTML('afterend', '<div class="form-hint" style="margin-top:4px">Agent key (' + esc(agentKey.key_prefix) + '...) was shown once at startup or when created. Check the controller logs or create a new one in Settings.</div>');
        } else {
            el.innerHTML = 'No agent key found. <a href="#" onclick="showPage(\'settings\');return false" style="color:var(--accent)">Create one in Settings</a> with role "agent".';
        }
    } catch (e) {
        el.textContent = 'KRONFORCE_AGENT_KEY=<agent_key> KRONFORCE_CONTROLLER_URL=' + host + ' cargo run --bin kronforce-agent';
    }
}

function copyToClipboard(text, successMsg) {
    if (navigator.clipboard && window.isSecureContext) {
        navigator.clipboard.writeText(text).then(() => toast(successMsg || 'Copied'));
    } else {
        // Fallback for non-secure contexts (http, 127.0.0.1)
        const textarea = document.createElement('textarea');
        textarea.value = text;
        textarea.style.position = 'fixed';
        textarea.style.opacity = '0';
        document.body.appendChild(textarea);
        textarea.select();
        document.execCommand('copy');
        document.body.removeChild(textarea);
        toast(successMsg || 'Copied');
    }
}

function copyPairCommand() {
    const el = document.getElementById('pair-cmd-text');
    if (el) {
        const text = el.dataset.fullCmd || el.textContent;
        copyToClipboard(text, 'Command copied — replace <paste_agent_key> with your full agent key');
    }
}

function copyKey() {
    const el = document.getElementById('new-key-value');
    if (el) {
        copyToClipboard(el.textContent, 'Key copied to clipboard');
    }
}

async function revokeKey(id, name) {
    if (!confirm('Revoke key "' + name + '"? This cannot be undone.')) return;
    try {
        await api('DELETE', '/api/keys/' + id);
        toast('Key revoked');
        fetchKeys();
    } catch (e) {
        toast(e.message, 'error');
    }
}

// --- Setup Wizard ---
let wizardStep = 0;
let wizardData = { jobCreated: null };
const WIZARD_STEPS = 5;

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
            '<p style="margin:8px 0;font-size:13px"><strong>&#128276; Notifications</strong> — Email and SMS alerts on failures</p>' +
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
        body.innerHTML =
            '<p style="color:var(--text-secondary);margin-bottom:12px">Agents run jobs on remote machines. You can skip this if running everything locally.</p>' +
            '<div style="background:var(--bg-primary);border:1px solid var(--border);border-radius:var(--radius);padding:12px;margin-bottom:12px">' +
            '<p style="font-size:11px;color:var(--text-muted);margin:0 0 6px">Standard agent (Rust binary):</p>' +
            '<code style="font-size:11px;word-break:break-all">KRONFORCE_AGENT_KEY=&lt;key&gt; KRONFORCE_CONTROLLER_URL=http://localhost:8080 cargo run --bin kronforce-agent</code>' +
            '</div>' +
            '<div style="background:var(--bg-primary);border:1px solid var(--border);border-radius:var(--radius);padding:12px;margin-bottom:12px">' +
            '<p style="font-size:11px;color:var(--text-muted);margin:0 0 6px">Custom agent (Python example):</p>' +
            '<code style="font-size:11px">KRONFORCE_AGENT_KEY=&lt;key&gt; python3 examples/custom_agent.py</code>' +
            '</div>' +
            '<div style="background:rgba(62,139,255,0.08);border:1px solid var(--accent);border-radius:var(--radius);padding:10px;font-size:11px;color:var(--text-secondary)">' +
            '<strong>Agent Key Required:</strong> Find your agent key in Settings &rarr; Agent Authentication. All agents must include this key to connect.' +
            '</div>' +
            '<p style="font-size:12px;color:var(--text-muted);margin-top:12px">See the <a href="#" onclick="closeWizard();showPage(\'docs\');return false" style="color:var(--accent)">Docs</a> page for the full agent protocol.</p>';
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

