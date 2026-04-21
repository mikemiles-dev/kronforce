// Kronforce — Connections page

let allConnections = [];
let editingConnectionName = null;

const CONNECTION_FIELDS = {
    postgres: [
        { name: 'connection_string', label: 'Connection String', type: 'password', placeholder: 'postgresql://user:pass@host:5432/dbname', required: true },
    ],
    mysql: [
        { name: 'connection_string', label: 'Connection String', type: 'password', placeholder: 'mysql://user:pass@host:3306/dbname', required: true },
    ],
    sqlite: [
        { name: 'connection_string', label: 'Database Path', type: 'text', placeholder: '/path/to/database.db', required: true },
    ],
    ftp: [
        { name: 'host', label: 'Host', type: 'text', placeholder: 'ftp.example.com', required: true },
        { name: 'port', label: 'Port', type: 'number', placeholder: '21' },
        { name: 'username', label: 'Username', type: 'text', placeholder: 'ftpuser', required: true },
        { name: 'password', label: 'Password', type: 'password', placeholder: '' },
    ],
    sftp: [
        { name: 'host', label: 'Host', type: 'text', placeholder: 'sftp.example.com', required: true },
        { name: 'port', label: 'Port', type: 'number', placeholder: '22' },
        { name: 'username', label: 'Username', type: 'text', placeholder: 'sftpuser', required: true },
        { name: 'password', label: 'Password', type: 'password', placeholder: '' },
        { name: 'private_key', label: 'Private Key (PEM)', type: 'textarea', placeholder: '-----BEGIN RSA PRIVATE KEY-----\n...' },
    ],
    http: [
        { name: 'base_url', label: 'Base URL', type: 'text', placeholder: 'https://api.example.com', required: true },
        { name: 'auth_type', label: 'Auth Type', type: 'select', options: [
            { value: 'none', label: 'None' },
            { value: 'bearer', label: 'Bearer Token' },
            { value: 'basic', label: 'Basic Auth' },
            { value: 'header', label: 'Custom Header' },
        ]},
        { name: 'token', label: 'Bearer Token', type: 'password', placeholder: 'your-api-token', showIf: { auth_type: 'bearer' } },
        { name: 'username', label: 'Username', type: 'text', placeholder: '', showIf: { auth_type: 'basic' } },
        { name: 'password', label: 'Password', type: 'password', placeholder: '', showIf: { auth_type: 'basic' } },
        { name: 'header_name', label: 'Header Name', type: 'text', placeholder: 'X-API-Key', showIf: { auth_type: 'header' } },
        { name: 'header_value', label: 'Header Value', type: 'password', placeholder: '', showIf: { auth_type: 'header' } },
    ],
    kafka: [
        { name: 'broker', label: 'Broker', type: 'text', placeholder: 'localhost:9092', required: true },
        { name: 'username', label: 'SASL Username', type: 'text', placeholder: '' },
        { name: 'password', label: 'SASL Password', type: 'password', placeholder: '' },
    ],
    mqtt: [
        { name: 'broker', label: 'Broker', type: 'text', placeholder: 'localhost', required: true },
        { name: 'port', label: 'Port', type: 'number', placeholder: '1883' },
        { name: 'username', label: 'Username', type: 'text', placeholder: '' },
        { name: 'password', label: 'Password', type: 'password', placeholder: '' },
        { name: 'client_id', label: 'Client ID', type: 'text', placeholder: 'kronforce-client' },
    ],
    rabbitmq: [
        { name: 'url', label: 'Connection URL', type: 'password', placeholder: 'amqp://user:pass@localhost:5672/', required: true },
    ],
    redis: [
        { name: 'url', label: 'Connection URL', type: 'password', placeholder: 'redis://localhost:6379', required: true },
        { name: 'password', label: 'Password', type: 'password', placeholder: '' },
    ],
    mongodb: [
        { name: 'connection_string', label: 'Connection String', type: 'password', placeholder: 'mongodb://user:pass@host:27017/dbname', required: true },
    ],
    ssh: [
        { name: 'host', label: 'Host', type: 'text', placeholder: 'server.example.com', required: true },
        { name: 'port', label: 'Port', type: 'number', placeholder: '22' },
        { name: 'username', label: 'Username', type: 'text', placeholder: 'deploy', required: true },
        { name: 'password', label: 'Password', type: 'password', placeholder: '' },
        { name: 'private_key', label: 'Private Key (PEM)', type: 'textarea', placeholder: '-----BEGIN RSA PRIVATE KEY-----\n...' },
    ],
    smtp: [
        { name: 'host', label: 'SMTP Host', type: 'text', placeholder: 'smtp.gmail.com', required: true },
        { name: 'port', label: 'Port', type: 'number', placeholder: '587' },
        { name: 'username', label: 'Username', type: 'text', placeholder: '' },
        { name: 'password', label: 'Password', type: 'password', placeholder: '' },
    ],
    s3: [
        { name: 'endpoint', label: 'Endpoint', type: 'text', placeholder: 'https://s3.amazonaws.com', required: true },
        { name: 'bucket', label: 'Bucket', type: 'text', placeholder: 'my-bucket' },
        { name: 'region', label: 'Region', type: 'text', placeholder: 'us-east-1' },
        { name: 'access_key', label: 'Access Key', type: 'password', placeholder: '' },
        { name: 'secret_key', label: 'Secret Key', type: 'password', placeholder: '' },
    ],
};

const TYPE_LABELS = {
    postgres: 'PostgreSQL', mysql: 'MySQL', sqlite: 'SQLite',
    ftp: 'FTP', sftp: 'SFTP', http: 'HTTP',
    kafka: 'Kafka', mqtt: 'MQTT', rabbitmq: 'RabbitMQ', redis: 'Redis',
    mongodb: 'MongoDB', ssh: 'SSH', smtp: 'SMTP', s3: 'S3',
};

async function fetchConnections() {
    try {
        const conns = await api('GET', '/api/connections');
        allConnections = conns;
        renderConnections();
    } catch (e) {
        document.getElementById('connections-list').innerHTML = '<div style="text-align:center;color:var(--danger);padding:24px">Error loading connections: ' + esc(e.message) + '</div>';
    }
}

function renderConnections() {
    const container = document.getElementById('connections-list');
    if (allConnections.length === 0) {
        container.innerHTML = renderRichEmptyState({
            icon: '&#128279;',
            title: 'No connections yet',
            description: 'Connections store credentials for databases, APIs, and services. Jobs reference them by name instead of embedding passwords.',
            actions: [{ label: 'Create Connection', onclick: 'showAddConnectionForm()', primary: true }],
        });
        return;
    }

    let html = '<table class="data-table"><thead><tr>';
    html += '<th>Name</th><th>Type</th><th>Description</th><th>Updated</th><th>Actions</th>';
    html += '</tr></thead><tbody>';

    for (const c of allConnections) {
        html += '<tr>';
        html += '<td><strong>' + esc(c.name) + '</strong></td>';
        html += '<td><span class="badge">' + esc(TYPE_LABELS[c.conn_type] || c.conn_type) + '</span></td>';
        html += '<td style="color:var(--text-muted)">' + esc(c.description || '') + '</td>';
        html += '<td style="white-space:nowrap">' + fmtDate(c.updated_at) + '</td>';
        html += '<td style="white-space:nowrap">';
        html += '<button class="btn btn-ghost btn-sm" onclick="testExistingConnection(\'' + esc(c.name) + '\')">Test</button>';
        html += '<button class="btn btn-ghost btn-sm" onclick="editConnection(\'' + esc(c.name) + '\')">Edit</button>';
        html += '<button class="btn btn-ghost btn-sm" style="color:var(--danger)" onclick="deleteConnection(\'' + esc(c.name) + '\')">Delete</button>';
        html += '</td>';
        html += '</tr>';
    }

    html += '</tbody></table>';
    container.innerHTML = html;
}

function showAddConnectionForm(existingName) {
    editingConnectionName = existingName || null;
    document.getElementById('connection-form').style.display = '';
    document.getElementById('connection-form-title').textContent = existingName ? 'Edit Connection' : 'New Connection';
    document.getElementById('conn-name').value = '';
    document.getElementById('conn-name').disabled = false;
    document.getElementById('conn-type').value = 'postgres';
    document.getElementById('conn-description').value = '';
    document.getElementById('conn-test-result').textContent = '';
    onConnectionTypeChange();

    if (existingName) {
        const c = allConnections.find(x => x.name === existingName);
        if (c) {
            document.getElementById('conn-name').value = c.name;
            document.getElementById('conn-name').disabled = true;
            document.getElementById('conn-type').value = c.conn_type;
            document.getElementById('conn-description').value = c.description || '';
            onConnectionTypeChange();
            // Fill config fields
            if (c.config && typeof c.config === 'object') {
                for (const [k, v] of Object.entries(c.config)) {
                    const el = document.getElementById('conn-cfg-' + k);
                    if (el) el.value = v;
                }
            }
        }
    }
}

function hideAddConnectionForm() {
    document.getElementById('connection-form').style.display = 'none';
    editingConnectionName = null;
}

function onConnectionTypeChange() {
    const type = document.getElementById('conn-type').value;
    const fields = CONNECTION_FIELDS[type] || [];
    const container = document.getElementById('conn-config-fields');
    let html = '';

    for (const f of fields) {
        const id = 'conn-cfg-' + f.name;
        let style = '';
        if (f.showIf) {
            // Initially hidden; shown by updateConnectionFieldVisibility
            style = 'display:none';
        }
        html += '<div class="form-group conn-conditional-field" data-field="' + f.name + '" style="margin-bottom:8px;' + style + '">';
        html += '<label>' + esc(f.label) + (f.required ? ' *' : '') + '</label>';

        if (f.type === 'select') {
            html += '<select id="' + id + '" onchange="updateConnectionFieldVisibility()" style="max-width:300px">';
            for (const opt of (f.options || [])) {
                html += '<option value="' + esc(opt.value) + '">' + esc(opt.label) + '</option>';
            }
            html += '</select>';
        } else if (f.type === 'textarea') {
            html += '<textarea id="' + id + '" rows="3" placeholder="' + esc(f.placeholder || '') + '" style="font-family:monospace;font-size:12px;max-width:500px"></textarea>';
        } else {
            html += '<input id="' + id + '" type="' + f.type + '" placeholder="' + esc(f.placeholder || '') + '" style="max-width:500px">';
        }
        html += '</div>';
    }

    container.innerHTML = html;
    updateConnectionFieldVisibility();
}

function updateConnectionFieldVisibility() {
    const type = document.getElementById('conn-type').value;
    const fields = CONNECTION_FIELDS[type] || [];

    for (const f of fields) {
        if (!f.showIf) continue;
        const el = document.querySelector('.conn-conditional-field[data-field="' + f.name + '"]');
        if (!el) continue;
        let visible = true;
        for (const [depField, depValue] of Object.entries(f.showIf)) {
            const depEl = document.getElementById('conn-cfg-' + depField);
            if (depEl && depEl.value !== depValue) visible = false;
        }
        el.style.display = visible ? '' : 'none';
    }
}

function getConnectionConfig() {
    const type = document.getElementById('conn-type').value;
    const fields = CONNECTION_FIELDS[type] || [];
    const config = {};
    for (const f of fields) {
        const el = document.getElementById('conn-cfg-' + f.name);
        if (el && el.value) {
            if (f.type === 'number') {
                config[f.name] = parseInt(el.value) || 0;
            } else {
                config[f.name] = el.value;
            }
        }
    }
    return config;
}

async function saveConnection() {
    const name = document.getElementById('conn-name').value.trim();
    const connType = document.getElementById('conn-type').value;
    const description = document.getElementById('conn-description').value.trim() || null;
    const config = getConnectionConfig();

    if (!name) { toast('Connection name is required', 'error'); return; }

    try {
        if (editingConnectionName) {
            await api('PUT', '/api/connections/' + encodeURIComponent(editingConnectionName), {
                conn_type: connType,
                description: description,
                config: config,
            });
            toast('Connection "' + name + '" updated', 'success');
        } else {
            await api('POST', '/api/connections', {
                name: name,
                conn_type: connType,
                description: description,
                config: config,
            });
            toast('Connection "' + name + '" created', 'success');
        }
        hideAddConnectionForm();
        fetchConnections();
    } catch (e) {
        toast('Error: ' + e.message, 'error');
    }
}

async function editConnection(name) {
    showAddConnectionForm(name);
}

async function deleteConnection(name) {
    if (!confirm('Delete connection "' + name + '"? Jobs referencing this connection will fail.')) return;
    try {
        await api('DELETE', '/api/connections/' + encodeURIComponent(name));
        toast('Connection "' + name + '" deleted', 'success');
        fetchConnections();
    } catch (e) {
        toast('Error: ' + e.message, 'error');
    }
}

async function testExistingConnection(name) {
    toast('Testing connection "' + name + '"...', 'success');
    try {
        const result = await api('POST', '/api/connections/' + encodeURIComponent(name) + '/test');
        if (result.success) {
            toast('Connection "' + name + '": ' + result.message, 'success');
        } else {
            toast('Connection "' + name + '" failed: ' + result.message, 'error');
        }
    } catch (e) {
        toast('Test failed: ' + e.message, 'error');
    }
}

async function testConnectionFromForm() {
    const name = editingConnectionName || document.getElementById('conn-name').value.trim();
    const resultEl = document.getElementById('conn-test-result');
    if (!name) { toast('Save the connection first, then test', 'error'); return; }

    // Save first if editing, then test
    if (editingConnectionName) {
        resultEl.textContent = 'Testing...';
        resultEl.style.color = 'var(--text-muted)';
        try {
            const result = await api('POST', '/api/connections/' + encodeURIComponent(name) + '/test');
            if (result.success) {
                resultEl.textContent = result.message;
                resultEl.style.color = 'var(--success)';
            } else {
                resultEl.textContent = result.message;
                resultEl.style.color = 'var(--danger)';
            }
        } catch (e) {
            resultEl.textContent = 'Error: ' + e.message;
            resultEl.style.color = 'var(--danger)';
        }
    } else {
        toast('Save the connection first, then use Test', 'error');
    }
}
