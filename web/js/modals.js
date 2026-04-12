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
        openModal('create-modal');
    } catch (e) {
        toast(e.message, 'error');
    }
}

function closeCreateModal() {
    closeModal('create-modal');
}

function toggleMqSection() {
    const sec = document.getElementById('mq-task-section');
    if (sec) sec.style.display = sec.style.display === 'none' ? '' : 'none';
}

function updateTaskFields() {
    const type = document.querySelector('input[name="task-type"]:checked').value;
    const allTaskFields = ['shell','http','sql','ftp','script','file_push','kafka','rabbitmq','mqtt','redis','mcp','kafka_consume','mqtt_subscribe','rabbitmq_consume','redis_read'];
    for (const t of allTaskFields) {
        const el = document.getElementById('task-' + t + '-fields');
        if (el) el.style.display = t === type ? '' : 'none';
    }
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

function cronFieldVal(mode, val) {
    if (mode === 'every') return '*';
    if (mode === 'step') return '*/' + (val || '1');
    return val || '*';
}

function buildCronFromUI() {
    const sec = cronFieldVal(
        document.getElementById('cb-sec-mode').value,
        document.getElementById('cb-sec-val').value
    );
    const min = cronFieldVal(
        document.getElementById('cb-min-mode').value,
        document.getElementById('cb-min-val').value
    );
    const hr = cronFieldVal(
        document.getElementById('cb-hr-mode').value,
        document.getElementById('cb-hr-val').value
    );
    const dom = cronFieldVal(
        document.getElementById('cb-dom-mode').value,
        document.getElementById('cb-dom-val').value
    );
    const mon = cronFieldVal(
        document.getElementById('cb-mon-mode').value,
        document.getElementById('cb-mon-val').value
    );

    const selectedDow = Array.from(document.querySelectorAll('.cron-dow.active')).map(b => b.dataset.dow);
    const dow = selectedDow.length > 0 ? selectedDow.join(',') : '*';

    const expr = sec + ' ' + min + ' ' + hr + ' ' + dom + ' ' + mon + ' ' + dow;

    // Build description
    const dayNames = {0:'Sun',1:'Mon',2:'Tue',3:'Wed',4:'Thu',5:'Fri',6:'Sat'};
    let parts = [];
    if (sec !== '0' && sec !== '*') parts.push('sec=' + sec);
    if (min === '*') parts.push('every minute');
    else if (min.startsWith('*/')) parts.push('every ' + min.slice(2) + ' min');
    else parts.push('at :' + String(min).padStart(2, '0'));
    if (hr !== '*') {
        if (hr.startsWith('*/')) parts.push('every ' + hr.slice(2) + ' hours');
        else if (hr.includes('-')) parts.push('hours ' + hr);
        else parts.push(String(hr).padStart(2, '0') + 'h');
    }
    if (dom !== '*') parts.push('day ' + dom);
    if (mon !== '*') parts.push('month ' + mon);
    if (dow !== '*') parts.push(selectedDow.map(d => dayNames[d]).join(','));

    document.getElementById('cb-preview').textContent = expr;
    document.getElementById('cb-description').textContent = parts.join(', ');
    document.getElementById('f-cron').value = expr;
}

function detectCronMode(val) {
    if (val === '*') return { mode: 'every', val: '*' };
    if (val.startsWith('*/')) return { mode: 'step', val: val.slice(2) };
    if (val.includes('-')) return { mode: 'range', val: val };
    return { mode: 'fixed', val: val };
}

function parseCronToUI(expr) {
    // Reset dow buttons
    document.querySelectorAll('.cron-dow').forEach(b => b.classList.remove('active'));

    if (!expr) {
        // Defaults
        document.getElementById('cb-sec-mode').value = 'fixed';
        document.getElementById('cb-sec-val').value = '0';
        document.getElementById('cb-min-mode').value = 'every';
        document.getElementById('cb-min-val').value = '*';
        document.getElementById('cb-hr-mode').value = 'every';
        document.getElementById('cb-hr-val').value = '*';
        document.getElementById('cb-dom-mode').value = 'every';
        document.getElementById('cb-dom-val').value = '*';
        document.getElementById('cb-mon-mode').value = 'every';
        document.getElementById('cb-mon-val').value = '*';
        buildCronFromUI();
        return;
    }
    const parts = expr.split(/\s+/);
    if (parts.length !== 6) { buildCronFromUI(); return; }

    const [sec, min, hr, dom, mon, dow] = parts;

    const s = detectCronMode(sec);
    document.getElementById('cb-sec-mode').value = s.mode;
    document.getElementById('cb-sec-val').value = s.val;

    const m = detectCronMode(min);
    document.getElementById('cb-min-mode').value = m.mode;
    document.getElementById('cb-min-val').value = m.val;

    const h = detectCronMode(hr);
    document.getElementById('cb-hr-mode').value = h.mode;
    document.getElementById('cb-hr-val').value = h.val;

    const d = detectCronMode(dom);
    document.getElementById('cb-dom-mode').value = d.mode;
    document.getElementById('cb-dom-val').value = d.val;

    const mo = detectCronMode(mon);
    document.getElementById('cb-mon-mode').value = mo.mode;
    document.getElementById('cb-mon-val').value = mo.val;

    if (dow !== '*') {
        dow.split(',').forEach(v => {
            const btn = document.querySelector('.cron-dow[data-dow="' + v.trim() + '"]');
            if (btn) btn.classList.add('active');
        });
    }

    buildCronFromUI();
}

function updateCronPreviewFromRaw() {
    // Just sync the raw input — don't update builder to avoid loops
}

// Legacy compat — old code called these
function updateCronOptions() {}

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
let eventJobFilterWired = false;
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
    const forward_url = document.getElementById('f-forward-url').value.trim() || null;
    if (extractions.length === 0 && triggers.length === 0 && assertions.length === 0 && !forward_url) return null;
    const rules = { extractions, triggers, assertions };
    if (forward_url) rules.forward_url = forward_url;
    return rules;
}

function populateOutputRules(rules) {
    document.getElementById('extractions-container').innerHTML = '';
    document.getElementById('triggers-container').innerHTML = '';
    document.getElementById('assertions-container').innerHTML = '';
    document.getElementById('f-forward-url').value = '';
    if (!rules) return;
    (rules.extractions || []).forEach(r => addExtractionRow(r.name, r.pattern, r.type, r.write_to_variable, r.target));
    (rules.triggers || []).forEach(t => addTriggerRow(t.pattern, t.severity));
    (rules.assertions || []).forEach(a => addAssertionRow(a.pattern, a.message));
    if (rules.forward_url) document.getElementById('f-forward-url').value = rules.forward_url;
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

// --- Job Parameters ---
let jobParamsDef = [];

function addJobParam() {
    jobParamsDef.push({ name: '', param_type: 'text', required: false, default: '', description: '' });
    renderJobParams();
}

function removeJobParam(idx) {
    jobParamsDef.splice(idx, 1);
    renderJobParams();
}

function renderJobParams() {
    const list = document.getElementById('job-params-list');
    if (!list) return;
    if (jobParamsDef.length === 0) { list.innerHTML = ''; return; }
    let html = '';
    for (let i = 0; i < jobParamsDef.length; i++) {
        const p = jobParamsDef[i];
        html += '<div style="display:flex;gap:6px;align-items:center;margin-bottom:6px;padding:8px;background:var(--bg-tertiary);border-radius:var(--radius);border:1px solid var(--border)">';
        html += '<input type="text" value="' + esc(p.name) + '" placeholder="name" style="width:80px;font-size:11px" onchange="jobParamsDef[' + i + '].name=this.value">';
        html += '<select style="width:80px;font-size:11px" onchange="jobParamsDef[' + i + '].param_type=this.value">';
        for (const t of ['text','number','select','boolean']) {
            html += '<option value="' + t + '"' + (p.param_type === t ? ' selected' : '') + '>' + t + '</option>';
        }
        html += '</select>';
        html += '<input type="text" value="' + esc(p.default || '') + '" placeholder="default" style="width:80px;font-size:11px" onchange="jobParamsDef[' + i + '].default=this.value">';
        html += '<label style="font-size:11px;display:flex;align-items:center;gap:3px;white-space:nowrap"><input type="checkbox"' + (p.required ? ' checked' : '') + ' onchange="jobParamsDef[' + i + '].required=this.checked"> Req</label>';
        html += '<input type="text" value="' + esc(p.description || '') + '" placeholder="description" style="flex:1;font-size:11px" onchange="jobParamsDef[' + i + '].description=this.value">';
        html += '<button class="btn btn-ghost btn-sm" style="color:var(--danger);padding:2px 6px;font-size:14px" onclick="removeJobParam(' + i + ')">&times;</button>';
        html += '</div>';
    }
    list.innerHTML = html;
}

function collectJobParams() {
    const valid = jobParamsDef.filter(p => p.name.trim());
    return valid.length > 0 ? valid : null;
}

function populateJobParams(params) {
    jobParamsDef = params ? JSON.parse(JSON.stringify(params)) : [];
    renderJobParams();
}

// --- Trigger with Params ---
let triggerParamsJobId = null;

function showTriggerParamsModal(jobId, params) {
    triggerParamsJobId = jobId;
    const content = document.getElementById('trigger-params-content');
    let html = '';
    for (const p of params) {
        html += '<div class="form-group" style="margin-bottom:10px">';
        html += '<label>' + esc(p.name) + (p.required ? ' <span style="color:var(--danger)">*</span>' : '') + '</label>';
        if (p.description) html += '<div class="form-hint">' + esc(p.description) + '</div>';
        if (p.param_type === 'boolean') {
            html += '<label style="font-size:12px;display:flex;align-items:center;gap:4px"><input type="checkbox" id="tp-' + esc(p.name) + '"' + (p.default === 'true' ? ' checked' : '') + '> Enabled</label>';
        } else if (p.param_type === 'select' && p.options && p.options.length) {
            html += '<select id="tp-' + esc(p.name) + '">';
            for (const opt of p.options) {
                html += '<option value="' + esc(opt) + '"' + (p.default === opt ? ' selected' : '') + '>' + esc(opt) + '</option>';
            }
            html += '</select>';
        } else if (p.param_type === 'number') {
            html += '<input type="number" id="tp-' + esc(p.name) + '" value="' + esc(p.default || '') + '">';
        } else {
            html += '<input type="text" id="tp-' + esc(p.name) + '" value="' + esc(p.default || '') + '" placeholder="' + esc(p.name) + '">';
        }
        html += '</div>';
    }
    content.innerHTML = html;
    openModal('trigger-params-modal');
}

async function submitTriggerWithParams() {
    if (!triggerParamsJobId) return;
    const job = allJobs.find(j => j.id === triggerParamsJobId);
    const params = {};
    if (job && job.parameters) {
        for (const p of job.parameters) {
            const el = document.getElementById('tp-' + p.name);
            if (!el) continue;
            if (p.param_type === 'boolean') {
                params[p.name] = el.checked ? 'true' : 'false';
            } else {
                params[p.name] = el.value;
            }
        }
    }
    closeModal('trigger-params-modal');
    const btn = document.getElementById('trigger-' + triggerParamsJobId);
    if (btn) btn.classList.add('trigger-pending');
    try {
        await api('POST', '/api/jobs/' + triggerParamsJobId + '/trigger', { params });
        toast('Job triggered with parameters', 'info');
        pollForResult(triggerParamsJobId);
    } catch (e) {
        toast(e.message, 'error');
        if (btn) btn.classList.remove('trigger-pending');
    }
}

// --- Dependency Map ---

let cyInstance = null;

async function renderMap() {
    let jobs;
    try {
        const res = await api('GET', '/api/jobs?per_page=1000');
        jobs = typeof applyJobFilters === 'function' ? applyJobFilters(res.data) : res.data;
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
                    'text-valign': 'center',
                    'text-halign': 'center',
                    'font-size': 13,
                    'font-family': '-apple-system, BlinkMacSystemFont, Segoe UI, Roboto, sans-serif',
                    'color': textColor,
                    'background-color': function(ele) { return statusColor(ele.data('lastStatus')); },
                    'background-opacity': 0.15,
                    'border-width': 3,
                    'border-color': function(ele) { return statusColor(ele.data('lastStatus')); },
                    'shape': 'round-rectangle',
                    'width': function(ele) { return Math.max(120, ele.data('label').length * 9 + 30); },
                    'height': 40,
                    'text-wrap': 'ellipsis',
                    'text-max-width': function(ele) { return Math.max(100, ele.data('label').length * 9 + 10); },
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

let miniCyInstance = null;

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
    if (!name) { toast('Key name is required', 'error'); return; }
    try {
        const body = { name, role };
        if (allowed_groups && allowed_groups.length > 0) body.allowed_groups = allowed_groups;
        const res = await api('POST', '/api/keys', body);
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
        if (k.allowed_groups && k.allowed_groups.length) {
            html += '<span style="font-size:10px;color:var(--text-muted)">' + k.allowed_groups.map(esc).join(', ') + '</span>';
        }
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

