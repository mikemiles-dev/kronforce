// Kronforce - Create/edit modal, task form, schedule, target, dependencies, notifications, keys, pair command

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

// Task types that support connection references
const CONNECTION_TASK_TYPES = ['sql', 'http', 'ftp', 'kafka', 'rabbitmq', 'mqtt', 'redis', 'kafka_consume', 'mqtt_subscribe', 'rabbitmq_consume', 'redis_read'];

function populateConnectionSelect(selectedConn) {
    const sel = document.getElementById('f-task-connection');
    if (!sel) return;
    sel.innerHTML = '<option value="">None — use inline credentials</option>';
    if (typeof allConnections !== 'undefined' && allConnections.length > 0) {
        for (const c of allConnections) {
            const selected = c.name === selectedConn ? ' selected' : '';
            sel.innerHTML += '<option value="' + esc(c.name) + '"' + selected + '>' + esc(c.name) + ' (' + esc(c.conn_type) + ')</option>';
        }
    }
}

function updateConnectionVisibility() {
    const taskType = document.querySelector('input[name="task-type"]:checked');
    const group = document.getElementById('task-connection-group');
    if (!group || !taskType) return;
    group.style.display = CONNECTION_TASK_TYPES.includes(taskType.value) ? '' : 'none';
}

// --- Create/Edit Modal ---
function resetJobForm() {
    editingJobId = null;
    editingJobUpdatedAt = null;
    resetJobTabs();
    filePushBase64 = '';
    filePushFilename = '';
    filePushSize = 0;
    selectedCustomAgentData = null;
    const titleEl = document.getElementById('modal-title') || document.getElementById('designer-title');
    if (titleEl) titleEl.textContent = 'Create Job';
    document.getElementById('f-name').value = '';
    document.getElementById('f-command').value = '';
    document.getElementById('f-working-dir').value = '';
    document.getElementById('f-run-as').value = '';
    populateTaskForm(null);
    parseCronToUI('');
    document.getElementById('f-desc').value = '';
    populateGroupSelect(groupFilter || '');
    document.getElementById('f-retry-max').value = '0';
    document.getElementById('f-retry-delay').value = '0';
    document.getElementById('f-retry-backoff').value = '1.0';
    document.getElementById('f-priority').value = '0';
    document.getElementById('f-approval-required').checked = false;
    document.getElementById('f-sla-deadline').value = '';
    document.getElementById('f-sla-warning').value = '0';
    document.getElementById('f-starts-at').value = '';
    document.getElementById('f-expires-at').value = '';
    document.getElementById('f-max-concurrent').value = '0';
    populateJobParams(null);
    setEventJobFilter('');
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
    // Load connections for the dropdown
    if (typeof fetchConnections === 'function' && (typeof allConnections === 'undefined' || allConnections.length === 0)) {
        api('GET', '/api/connections').then(c => { allConnections = c; populateConnectionSelect(''); }).catch(() => {});
    } else {
        populateConnectionSelect('');
    }
}

function openCreateModal() {
    resetJobForm();
    // Navigate to designer page (full-page editor)
    showPage('designer');
}

async function copyJob(id) {
    try {
        const job = await api('GET', '/api/jobs/' + id);
        // Open as a new job with copied data
        openCreateModal();
        (document.getElementById('modal-title') || document.getElementById('designer-title')).textContent = 'Copy Job';
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
        document.getElementById('f-priority').value = job.priority || 0;
        document.getElementById('f-approval-required').checked = job.approval_required || false;
        document.getElementById('f-sla-deadline').value = job.sla_deadline || '';
        document.getElementById('f-sla-warning').value = job.sla_warning_mins || 0;
        document.getElementById('f-starts-at').value = job.starts_at ? toLocalDatetimeString(new Date(job.starts_at)) : '';
        document.getElementById('f-expires-at').value = job.expires_at ? toLocalDatetimeString(new Date(job.expires_at)) : '';
        document.getElementById('f-max-concurrent').value = job.max_concurrent || 0;
        populateJobParams(job.parameters);
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
            setEventJobFilter(job.schedule.value.job_name_filter || '');
        } else if (schedType === 'calendar' && job.schedule.value) {
            populateCalendarFields(job.schedule.value);
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
        editingJobUpdatedAt = job.updated_at || null;
        if (typeof hideAiPrompt === 'function') hideAiPrompt();
        (document.getElementById('modal-title') || document.getElementById('designer-title')).textContent = 'Edit Job';
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
        document.getElementById('f-priority').value = job.priority || 0;
        document.getElementById('f-max-concurrent').value = job.max_concurrent || 0;
        populateJobParams(job.parameters);
        document.getElementById('f-approval-required').checked = job.approval_required || false;
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
            setEventJobFilter(job.schedule.value.job_name_filter || '');
        } else if (schedType === 'calendar' && job.schedule.value) {
            populateCalendarFields(job.schedule.value);
        } else if (schedType === 'interval' && job.schedule.value) {
            const el = document.getElementById('f-interval-secs');
            if (el) el.value = job.schedule.value.interval_secs || '';
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

        // Schedule window
        document.getElementById('f-starts-at').value = job.starts_at ? toLocalDatetimeString(new Date(job.starts_at)) : '';
        document.getElementById('f-expires-at').value = job.expires_at ? toLocalDatetimeString(new Date(job.expires_at)) : '';

        populateDeps(id, job.depends_on);
        populateOutputRules(job.output_rules);
        populateJobNotifications(job.notifications);
        document.getElementById('designer-title').textContent = 'Edit Job';
        showPage('designer');
    } catch (e) {
        toast(e.message, 'error');
    }
}

function closeCreateModal() {
    editingJobId = null;
    showPage('monitor');
}

function toggleMqSection() {
    const sec = document.getElementById('mq-task-section');
    if (!sec) return;
    const showing = sec.style.display === 'none';
    sec.style.display = showing ? '' : 'none';
    const btn = document.getElementById('mq-toggle-btn');
    if (btn) btn.classList.toggle('mq-active', showing);
    // Deselect top-level non-MQ radios so they unhighlight
    if (showing) {
        document.querySelectorAll('.radio-group input[name="task-type"]').forEach(function(r) {
            var mqTypes = ['kafka','rabbitmq','mqtt','redis','kafka_consume','rabbitmq_consume','mqtt_subscribe','redis_read'];
            if (!mqTypes.includes(r.value)) r.checked = false;
        });
    }
}

function updateTaskFields() {
    const type = document.querySelector('input[name="task-type"]:checked').value;
    const allTaskFields = ['shell','http','sql','ftp','script','docker_build','file_push','kafka','rabbitmq','mqtt','redis','mcp','kafka_consume','mqtt_subscribe','rabbitmq_consume','redis_read'];
    for (const t of allTaskFields) {
        const el = document.getElementById('task-' + t + '-fields');
        if (el) el.style.display = t === type ? '' : 'none';
    }
    // Manage message queues section and button highlight
    const mqTypes = ['kafka','rabbitmq','mqtt','redis','kafka_consume','rabbitmq_consume','mqtt_subscribe','redis_read'];
    const isMq = mqTypes.includes(type);
    const btn = document.getElementById('mq-toggle-btn');
    if (!isMq) {
        const sec = document.getElementById('mq-task-section');
        if (sec) sec.style.display = 'none';
        if (btn) btn.classList.remove('mq-active');
    } else {
        if (btn) btn.classList.add('mq-active');
    }
    if (type === 'script') populateScriptDropdown();
    if (type === 'docker_build') populateDockerScriptDropdown();
    updateConnectionVisibility();
}

var filePushBase64 = '';
var filePushFilename = '';
var filePushSize = 0;

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
        const task = { type: 'shell', command };
        const wd = document.getElementById('f-working-dir').value.trim();
        if (wd) task.working_dir = wd;
        return task;
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
        const connStr = document.getElementById('f-sql-conn').value.trim();
        const connRef = document.getElementById('f-task-connection').value;
        if (!query) return null;
        if (!connStr && !connRef) { toast('Provide a connection string or select a connection', 'error'); return null; }
        const task = { type: 'sql', driver: document.getElementById('f-sql-driver').value, query };
        if (connStr) task.connection_string = connStr;
        return task;
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
    if (type === 'docker_build') {
        const scriptName = document.getElementById('f-docker-script').value;
        if (!scriptName) { toast('Select a Dockerfile script', 'error'); return null; }
        const task = { type: 'docker_build', script_name: scriptName };
        const tag = document.getElementById('f-docker-tag').value.trim();
        if (tag) task.image_tag = tag;
        const args = document.getElementById('f-docker-args').value.trim();
        if (args) task.build_args = args;
        if (document.getElementById('f-docker-run').checked) task.run_after_build = true;
        return task;
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
    if (type === 'kafka_consume') {
        const broker = document.getElementById('f-kafkac-broker').value.trim();
        const topic = document.getElementById('f-kafkac-topic').value.trim();
        if (!broker || !topic) return null;
        const task = { type: 'kafka_consume', broker, topic };
        const group = document.getElementById('f-kafkac-group').value.trim();
        if (group) task.group_id = group;
        const max = parseInt(document.getElementById('f-kafkac-max').value);
        if (max > 1) task.max_messages = max;
        task.offset = document.getElementById('f-kafkac-offset').value;
        return task;
    }
    if (type === 'mqtt_subscribe') {
        const broker = document.getElementById('f-mqtts-broker').value.trim();
        const topic = document.getElementById('f-mqtts-topic').value.trim();
        if (!broker || !topic) return null;
        const task = { type: 'mqtt_subscribe', broker, topic };
        const port = parseInt(document.getElementById('f-mqtts-port').value);
        if (port && port !== 1883) task.port = port;
        const max = parseInt(document.getElementById('f-mqtts-max').value);
        if (max > 1) task.max_messages = max;
        task.qos = parseInt(document.getElementById('f-mqtts-qos').value);
        const user = document.getElementById('f-mqtts-user').value.trim();
        if (user) task.username = user;
        const pass = document.getElementById('f-mqtts-pass').value;
        if (pass) task.password = pass;
        return task;
    }
    if (type === 'rabbitmq_consume') {
        const url = document.getElementById('f-rmqc-url').value.trim();
        const queue = document.getElementById('f-rmqc-queue').value.trim();
        if (!url || !queue) return null;
        const task = { type: 'rabbitmq_consume', url, queue };
        const max = parseInt(document.getElementById('f-rmqc-max').value);
        if (max > 1) task.max_messages = max;
        return task;
    }
    if (type === 'redis_read') {
        const url = document.getElementById('f-redisr-url').value.trim();
        const key = document.getElementById('f-redisr-key').value.trim();
        if (!url || !key) return null;
        const task = { type: 'redis_read', url, key };
        task.mode = document.getElementById('f-redisr-mode').value;
        const count = parseInt(document.getElementById('f-redisr-count').value);
        if (count > 1) task.count = count;
        return task;
    }
    return null;
}

// Wrapper that injects the connection field into the built task
const _origBuildTaskFromForm = buildTaskFromForm;
buildTaskFromForm = function() {
    const task = _origBuildTaskFromForm();
    if (!task) return null;
    const connEl = document.getElementById('f-task-connection');
    if (connEl && connEl.value) {
        task.connection = connEl.value;
    }
    return task;
};

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
    if (radio) {
        radio.checked = true;
        // Auto-expand message queues section if a queue type is selected
        if (['kafka','rabbitmq','mqtt','redis','kafka_consume','rabbitmq_consume','mqtt_subscribe','redis_read'].includes(type)) {
            const sec = document.getElementById('mq-task-section');
            if (sec) sec.style.display = '';
        }
    }
    updateTaskFields();
    if (type === 'shell') {
        document.getElementById('f-command').value = task.command || '';
        document.getElementById('f-working-dir').value = task.working_dir || '';
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
    } else if (type === 'docker_build') {
        populateDockerScriptDropdown(task.script_name);
        document.getElementById('f-docker-tag').value = task.image_tag || '';
        document.getElementById('f-docker-args').value = task.build_args || '';
        document.getElementById('f-docker-run').checked = task.run_after_build || false;
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
    } else if (type === 'kafka_consume') {
        document.getElementById('f-kafkac-broker').value = task.broker || '';
        document.getElementById('f-kafkac-topic').value = task.topic || '';
        document.getElementById('f-kafkac-group').value = task.group_id || '';
        document.getElementById('f-kafkac-max').value = task.max_messages || 1;
        document.getElementById('f-kafkac-offset').value = task.offset || 'latest';
    } else if (type === 'mqtt_subscribe') {
        document.getElementById('f-mqtts-broker').value = task.broker || '';
        document.getElementById('f-mqtts-topic').value = task.topic || '';
        document.getElementById('f-mqtts-port').value = task.port || 1883;
        document.getElementById('f-mqtts-max').value = task.max_messages || 1;
        document.getElementById('f-mqtts-qos').value = task.qos != null ? task.qos : 0;
        document.getElementById('f-mqtts-user').value = task.username || '';
        document.getElementById('f-mqtts-pass').value = task.password || '';
    } else if (type === 'rabbitmq_consume') {
        document.getElementById('f-rmqc-url').value = task.url || '';
        document.getElementById('f-rmqc-queue').value = task.queue || '';
        document.getElementById('f-rmqc-max').value = task.max_messages || 1;
    } else if (type === 'redis_read') {
        document.getElementById('f-redisr-url').value = task.url || '';
        document.getElementById('f-redisr-key').value = task.key || '';
        document.getElementById('f-redisr-mode').value = task.mode || 'lpop';
        document.getElementById('f-redisr-count').value = task.count || 1;
    }
    // Populate connection dropdown
    populateConnectionSelect(task.connection || '');
    updateConnectionVisibility();
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

// toLocalDatetimeString defined in app.js

function updateSchedFields() {
    const type = document.querySelector('input[name="sched-type"]:checked').value;
    document.getElementById('cron-field').style.display = type === 'cron' ? '' : 'none';
    document.getElementById('oneshot-field').style.display = type === 'one_shot' ? '' : 'none';
    document.getElementById('event-field').style.display = type === 'event' ? '' : 'none';
    document.getElementById('calendar-field').style.display = type === 'calendar' ? '' : 'none';
    if (type === 'calendar') updateCalPreview();
}

function updateCalPreview() {
    const anchor = document.getElementById('f-cal-anchor').value;
    document.getElementById('cal-nth-fields').style.display = anchor === 'nth_weekday' ? '' : 'none';
    const offset = parseInt(document.getElementById('f-cal-offset').value) || 0;
    const hour = String(parseInt(document.getElementById('f-cal-hour').value) || 0).padStart(2, '0');
    const minute = String(parseInt(document.getElementById('f-cal-minute').value) || 0).padStart(2, '0');
    const months = Array.from(document.querySelectorAll('#cal-months .cron-dow.active')).map(b => parseInt(b.dataset.month));
    const monthNames = {1:'Jan',2:'Feb',3:'Mar',4:'Apr',5:'May',6:'Jun',7:'Jul',8:'Aug',9:'Sep',10:'Oct',11:'Nov',12:'Dec'};

    let desc = '';
    if (anchor === 'last_day') desc = 'Last day of month';
    else if (anchor.startsWith('day_')) desc = 'Day ' + anchor.slice(4);
    else if (anchor === 'nth_weekday') {
        const nth = document.getElementById('f-cal-nth').value;
        const wd = document.getElementById('f-cal-weekday').value;
        const ordinal = {1:'1st',2:'2nd',3:'3rd',4:'4th'}[nth] || nth + 'th';
        desc = ordinal + ' ' + wd.charAt(0).toUpperCase() + wd.slice(1);
    } else {
        desc = anchor.replace('_', ' ').replace(/\b\w/g, c => c.toUpperCase());
    }
    if (offset > 0) desc += ' + ' + offset + ' days';
    else if (offset < 0) desc += ' - ' + Math.abs(offset) + ' days';
    desc += ' at ' + hour + ':' + minute + ' UTC';
    if (months.length > 0 && months.length < 12) {
        desc += ' in ' + months.map(m => monthNames[m]).join(', ');
    }
    document.getElementById('cal-preview').textContent = desc;
}

function buildCalendarSchedule() {
    const anchor = document.getElementById('f-cal-anchor').value;
    const cal = {
        anchor: anchor,
        offset_days: parseInt(document.getElementById('f-cal-offset').value) || 0,
        hour: parseInt(document.getElementById('f-cal-hour').value) || 0,
        minute: parseInt(document.getElementById('f-cal-minute').value) || 0,
    };
    if (anchor === 'nth_weekday') {
        cal.nth = parseInt(document.getElementById('f-cal-nth').value) || 1;
        cal.weekday = document.getElementById('f-cal-weekday').value;
    }
    const months = Array.from(document.querySelectorAll('#cal-months .cron-dow.active')).map(b => parseInt(b.dataset.month));
    if (months.length > 0 && months.length < 12) cal.months = months;
    else cal.months = [];
    return cal;
}

function populateCalendarFields(cal) {
    if (!cal) return;
    document.getElementById('f-cal-anchor').value = cal.anchor || 'last_day';
    document.getElementById('f-cal-offset').value = cal.offset_days || 0;
    document.getElementById('f-cal-hour').value = cal.hour || 0;
    document.getElementById('f-cal-minute').value = cal.minute || 0;
    if (cal.nth) document.getElementById('f-cal-nth').value = cal.nth;
    if (cal.weekday) document.getElementById('f-cal-weekday').value = cal.weekday;
    document.querySelectorAll('#cal-months .cron-dow').forEach(b => b.classList.remove('active'));
    if (cal.months) {
        for (const m of cal.months) {
            const btn = document.querySelector('#cal-months .cron-dow[data-month="' + m + '"]');
            if (btn) btn.classList.add('active');
        }
    }
    updateCalPreview();
}

var currentExecMode = 'standard';
var selectedCustomAgentData = null;

function switchJobTab(tabId, btn) {
    document.querySelectorAll('.modal-tab-content').forEach(el => {
        if (el.id.startsWith('job-tab-')) el.classList.remove('active');
    });
    document.querySelectorAll('.modal-tabs .modal-tab').forEach(t => t.classList.remove('active'));
    document.querySelectorAll('.designer-steps .designer-step').forEach(t => t.classList.remove('active'));
    document.getElementById('job-tab-' + tabId).classList.add('active');
    if (btn) btn.classList.add('active');
}

function resetJobTabs() {
    document.querySelectorAll('.modal-tabs .modal-tab').forEach((t, i) => {
        t.classList.toggle('active', i === 0);
    });
    document.querySelectorAll('.designer-steps .designer-step').forEach((t, i) => {
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

    // Resolve initial job name
    const selectedJob = jobId ? jobs.find(j => j.id === jobId) : null;
    const selectedName = selectedJob ? selectedJob.name : '';

    let html = '<div class="dep-search-wrap" style="position:relative;flex:1;min-width:150px">';
    html += '<input type="hidden" class="dep-job-select" value="' + (jobId || '') + '">';
    if (selectedJob) {
        html += '<div class="dep-chip">';
        html += '<span>' + esc(selectedName) + '</span>';
        html += '<button type="button" onclick="clearDepSelection(this)">&times;</button>';
        html += '</div>';
        html += '<input type="text" class="dep-search-input" placeholder="Search jobs..." style="display:none">';
    } else {
        html += '<input type="text" class="dep-search-input" placeholder="Search jobs..." autocomplete="off">';
    }
    html += '<div class="dep-search-results" style="display:none"></div>';
    html += '</div>';
    html += '<div class="dep-window-group">';
    html += '<span class="dep-window-label">within</span>';
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

    // Wire up search
    const input = div.querySelector('.dep-search-input');
    const results = div.querySelector('.dep-search-results');
    const hidden = div.querySelector('.dep-job-select');

    if (input) {
        input.addEventListener('input', function() {
            const q = this.value.toLowerCase().trim();
            if (!q) { results.style.display = 'none'; return; }
            const alreadySelected = Array.from(document.querySelectorAll('#deps-entries .dep-job-select')).map(el => el.value).filter(Boolean);
            const matches = jobs.filter(j => j.name.toLowerCase().includes(q) && !alreadySelected.includes(j.id)).slice(0, 10);
            if (matches.length === 0) {
                results.innerHTML = '<div class="dep-search-item" style="color:var(--text-muted)">No matches</div>';
            } else {
                results.innerHTML = matches.map(j =>
                    '<div class="dep-search-item" data-id="' + j.id + '" data-name="' + esc(j.name) + '">' +
                    esc(j.name) + (j.group ? ' <span style="color:var(--text-muted);font-size:10px">' + esc(j.group) + '</span>' : '') +
                    '</div>'
                ).join('');
            }
            results.style.display = '';
        });
        input.addEventListener('focus', function() { if (this.value.trim()) this.dispatchEvent(new Event('input')); });
        input.addEventListener('keydown', function(e) {
            if (e.key === 'Escape') { results.style.display = 'none'; }
        });

        results.addEventListener('mousedown', function(e) {
            const item = e.target.closest('.dep-search-item');
            if (!item || !item.dataset.id) return;
            e.preventDefault();
            hidden.value = item.dataset.id;
            input.style.display = 'none';
            results.style.display = 'none';
            // Show chip
            const chip = document.createElement('div');
            chip.className = 'dep-chip';
            chip.innerHTML = '<span>' + esc(item.dataset.name) + '</span><button type="button" onclick="clearDepSelection(this)">&times;</button>';
            input.parentElement.insertBefore(chip, input);
        });

        // Close on outside click
        input.addEventListener('blur', function() { setTimeout(() => { results.style.display = 'none'; }, 150); });
    }

    updateDepsEmpty();
}

function clearDepSelection(btn) {
    const wrap = btn.closest('.dep-search-wrap');
    const hidden = wrap.querySelector('.dep-job-select') || wrap.querySelector('input[type="hidden"]');
    const input = wrap.querySelector('.dep-search-input');
    const chip = wrap.querySelector('.dep-chip');
    if (hidden) hidden.value = '';
    if (chip) chip.remove();
    if (input) { input.value = ''; input.style.display = ''; input.focus(); }
}

// --- Event Job Filter Search ---
var eventJobFilterWired = false;
function wireEventJobFilter() {
    if (eventJobFilterWired) return;
    eventJobFilterWired = true;
    const input = document.getElementById('event-job-search');
    const results = document.getElementById('event-job-results');
    const hidden = document.getElementById('f-event-job-filter');
    if (!input || !results || !hidden) return;

    input.addEventListener('input', function() {
        const q = this.value.toLowerCase().trim();
        if (!q) { results.style.display = 'none'; return; }
        const exclude = editingJobId;
        const matches = allJobs.filter(j => j.id !== exclude && j.name.toLowerCase().includes(q)).slice(0, 10);
        if (matches.length === 0) {
            results.innerHTML = '<div class="dep-search-item" style="color:var(--text-muted)">No matches</div>';
        } else {
            results.innerHTML = matches.map(j =>
                '<div class="dep-search-item" data-name="' + esc(j.name) + '">' +
                esc(j.name) + (j.group ? ' <span style="color:var(--text-muted);font-size:10px">' + esc(j.group) + '</span>' : '') +
                '</div>'
            ).join('');
        }
        results.style.display = '';
    });
    input.addEventListener('focus', function() { if (this.value.trim()) this.dispatchEvent(new Event('input')); });
    input.addEventListener('keydown', function(e) { if (e.key === 'Escape') results.style.display = 'none'; });
    input.addEventListener('blur', function() { setTimeout(() => { results.style.display = 'none'; }, 150); });

    results.addEventListener('mousedown', function(e) {
        const item = e.target.closest('.dep-search-item');
        if (!item || !item.dataset.name) return;
        e.preventDefault();
        hidden.value = item.dataset.name;
        input.style.display = 'none';
        results.style.display = 'none';
        const wrap = document.getElementById('event-job-filter-wrap');
        // Remove old chip
        const old = wrap.querySelector('.dep-chip');
        if (old) old.remove();
        const chip = document.createElement('div');
        chip.className = 'dep-chip';
        chip.innerHTML = '<span>' + esc(item.dataset.name) + '</span><button type="button" onclick="clearDepSelection(this)">&times;</button>';
        wrap.insertBefore(chip, input);
    });
}

function setEventJobFilter(jobName) {
    wireEventJobFilter();
    const wrap = document.getElementById('event-job-filter-wrap');
    const hidden = document.getElementById('f-event-job-filter');
    const input = document.getElementById('event-job-search');
    if (!wrap || !hidden || !input) return;
    // Clear old chip
    const old = wrap.querySelector('.dep-chip');
    if (old) old.remove();
    hidden.value = jobName || '';
    if (jobName) {
        input.style.display = 'none';
        const chip = document.createElement('div');
        chip.className = 'dep-chip';
        chip.innerHTML = '<span>' + esc(jobName) + '</span><button type="button" onclick="clearDepSelection(this)">&times;</button>';
        wrap.insertBefore(chip, input);
    } else {
        input.value = '';
        input.style.display = '';
    }
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
    } else if (schedType === 'calendar') {
        schedule = { type: 'calendar', value: buildCalendarSchedule() };
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
    const priority = parseInt(document.getElementById('f-priority').value) || 0;
    if (priority !== 0) body.priority = priority;
    const maxConcurrent = parseInt(document.getElementById('f-max-concurrent').value) || 0;
    if (maxConcurrent > 0) body.max_concurrent = maxConcurrent;
    const jobParams = collectJobParams();
    if (jobParams) body.parameters = jobParams;
    if (document.getElementById('f-approval-required').checked) body.approval_required = true;
    const slaDeadline = document.getElementById('f-sla-deadline').value.trim();
    if (slaDeadline) {
        body.sla_deadline = slaDeadline;
        const slaWarning = parseInt(document.getElementById('f-sla-warning').value) || 0;
        if (slaWarning > 0) body.sla_warning_mins = slaWarning;
    }
    const startsAt = document.getElementById('f-starts-at').value;
    if (startsAt) body.starts_at = new Date(startsAt).toISOString();
    const expiresAt = document.getElementById('f-expires-at').value;
    if (expiresAt) body.expires_at = new Date(expiresAt).toISOString();

    try {
        let jobId;
        if (editingJobId) {
            if (editingJobUpdatedAt) body.if_unmodified_since = editingJobUpdatedAt;
            await api('PUT', '/api/jobs/' + editingJobId, body);
            toast('Job updated');
            jobId = editingJobId;
        } else {
            const result = await api('POST', '/api/jobs', body);
            toast('Job created');
            jobId = result.id;
        }
        closeCreateModal();
        if (jobId) {
            showPage('monitor');
            showJobDetail(jobId);
        } else {
            showPage('monitor');
        }
    } catch (e) {
        if (e.message && e.message.includes('modified by another user')) {
            toast('Conflict: this job was edited by someone else. Reloading latest version...', 'error');
            if (editingJobId) {
                setTimeout(function() { openEditModal(editingJobId); }, 1500);
            }
        } else {
            toast(e.message, 'error');
        }
    }
}

function collectJobNotifications() {
    const onFailure = document.getElementById('f-notif-failure').checked;
    const onSuccess = document.getElementById('f-notif-success').checked;
    const onAssertion = document.getElementById('f-notif-assertion').checked;
    const emailOutput = document.getElementById('f-email-output').value || null;
    if (!onFailure && !onSuccess && !onAssertion && !emailOutput) return null;
    const emailsStr = document.getElementById('f-notif-emails').value.trim();
    const config = { on_failure: onFailure, on_success: onSuccess, on_assertion_failure: onAssertion };
    if (emailsStr) {
        config.recipients = { emails: emailsStr.split(',').map(s => s.trim()).filter(Boolean), phones: [] };
    }
    if (emailOutput) config.email_output = emailOutput;
    return config;
}

function populateJobNotifications(notif) {
    document.getElementById('f-notif-failure').checked = notif ? notif.on_failure : false;
    document.getElementById('f-notif-success').checked = notif ? notif.on_success : false;
    document.getElementById('f-notif-assertion').checked = notif ? notif.on_assertion_failure : false;
    const emails = notif && notif.recipients ? (notif.recipients.emails || []).join(', ') : '';
    document.getElementById('f-notif-emails').value = emails;
    document.getElementById('f-email-output').value = notif && notif.email_output ? notif.email_output : '';
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
    const groupsStr = document.getElementById('new-key-groups').value.trim();
    const allowed_groups = groupsStr ? groupsStr.split(',').map(s => s.trim()).filter(Boolean) : null;
    const expiresDays = document.getElementById('new-key-expires').value;
    if (!name) { toast('Key name is required', 'error'); return; }
    try {
        const body = { name, role };
        if (allowed_groups && allowed_groups.length > 0) body.allowed_groups = allowed_groups;
        if (expiresDays) {
            const d = new Date();
            d.setDate(d.getDate() + parseInt(expiresDays));
            body.expires_at = d.toISOString();
        }
        const res = await api('POST', '/api/keys', body);
        document.getElementById('new-key-display').style.display = '';
        const rawKey = res.raw_key;
        document.getElementById('new-key-display').innerHTML =
            '<strong>Key created!</strong> Copy it now — it won\'t be shown again.' +
            '<code id="new-key-value">' + esc(rawKey) + '</code>' +
            '<button class="btn btn-ghost btn-sm" onclick="copyKey()" style="margin-top:4px">&#128203; Copy to Clipboard</button>';
        document.getElementById('new-key-name').value = '';
        document.getElementById('new-key-expires').value = '';
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
        if (k.allowed_groups && k.allowed_groups.length) {
            html += '<span style="font-size:10px;color:var(--text-muted)">' + k.allowed_groups.map(esc).join(', ') + '</span>';
        }
        html += '<span class="time-text">' + (k.last_used_at ? 'used ' + fmtDate(k.last_used_at) : 'never used') + '</span>';
        if (k.expires_at) {
            const expired = new Date(k.expires_at) < new Date();
            html += '<span style="font-size:10px;color:' + (expired ? 'var(--danger)' : 'var(--text-muted)') + '">' + (expired ? 'expired ' : 'expires ') + fmtDate(k.expires_at) + '</span>';
        }
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
    const wrap = document.getElementById('pair-command-box');
    if (!wrap) return;
    const host = getControllerUrl();
    let agentKey = null;
    try {
        const keys = await api('GET', '/api/keys');
        agentKey = keys.find(k => k.role === 'agent' && k.active);
    } catch (e) { /* ignore */ }

    const keyPlaceholder = '<your_agent_key>';
    const keyHint = agentKey
        ? 'Agent key <code>' + esc(agentKey.key_prefix) + '...</code> exists — the full key was shown at creation. Check controller logs or <a href="#" onclick="showPage(\'settings\');return false" style="color:var(--accent)">create a new one</a>.'
        : 'No agent key found. <a href="#" onclick="showPage(\'settings\');return false" style="color:var(--accent)">Create one in Settings</a> with role "agent".';

    const binaryCmd = 'KRONFORCE_AGENT_KEY=' + keyPlaceholder + ' \\\nKRONFORCE_CONTROLLER_URL=' + host + ' \\\nKRONFORCE_AGENT_NAME=my-agent \\\nKRONFORCE_AGENT_TAGS=linux \\\n  ./kronforce-agent';
    const dockerCmd = 'docker run -d \\\n  -e KRONFORCE_AGENT_KEY=' + keyPlaceholder + ' \\\n  -e KRONFORCE_CONTROLLER_URL=' + host + ' \\\n  -e KRONFORCE_AGENT_NAME=my-agent \\\n  -e KRONFORCE_AGENT_TAGS=linux \\\n  ghcr.io/mikemiles-dev/kronforce:latest \\\n  kronforce-agent';

    let html = '<div class="pair-command">';
    html += '<div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:8px">';
    html += '<span class="pair-label">Connect an Agent</span>';
    html += '<div style="display:flex;gap:4px">';
    html += '<button class="btn btn-ghost btn-sm pair-tab-btn active" id="pair-tab-binary" onclick="switchPairTab(\'binary\')" style="font-size:11px;padding:3px 8px">Binary</button>';
    html += '<button class="btn btn-ghost btn-sm pair-tab-btn" id="pair-tab-docker" onclick="switchPairTab(\'docker\')" style="font-size:11px;padding:3px 8px">Docker</button>';
    html += '</div>';
    html += '</div>';
    html += '<pre class="pair-cmd-pre" id="pair-cmd-binary" style="margin:0 0 8px;background:var(--bg-tertiary);padding:10px 12px;border-radius:6px;font-size:11px;overflow-x:auto;white-space:pre-wrap;word-break:break-all;border:1px solid var(--border)">' + esc(binaryCmd) + '</pre>';
    html += '<pre class="pair-cmd-pre" id="pair-cmd-docker" style="display:none;margin:0 0 8px;background:var(--bg-tertiary);padding:10px 12px;border-radius:6px;font-size:11px;overflow-x:auto;white-space:pre-wrap;word-break:break-all;border:1px solid var(--border)">' + esc(dockerCmd) + '</pre>';
    html += '<div style="display:flex;align-items:center;justify-content:space-between">';
    html += '<div style="font-size:11px;color:var(--text-muted)">' + keyHint + '</div>';
    html += '<button class="btn btn-ghost btn-sm" onclick="copyPairCommand()" style="flex-shrink:0">Copy</button>';
    html += '</div>';
    html += '</div>';

    wrap.innerHTML = html;
}

function switchPairTab(tab) {
    document.querySelectorAll('.pair-cmd-pre').forEach(el => el.style.display = 'none');
    document.querySelectorAll('.pair-tab-btn').forEach(el => el.classList.remove('active'));
    const pre = document.getElementById('pair-cmd-' + tab);
    const btn = document.getElementById('pair-tab-' + tab);
    if (pre) pre.style.display = '';
    if (btn) btn.classList.add('active');
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
    const visible = document.querySelector('.pair-cmd-pre:not([style*="display: none"])') || document.querySelector('.pair-cmd-pre');
    if (visible) {
        copyToClipboard(visible.textContent, 'Command copied — replace <your_agent_key> with your full agent key');
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
