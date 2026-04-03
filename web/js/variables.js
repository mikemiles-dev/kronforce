// Kronforce - Variables page
// --- Variables ---
let allVariables = [];

async function fetchVariables() {
    try {
        allVariables = await api('GET', '/api/variables');
        renderVariables();
    } catch (e) {
        console.error('Failed to fetch variables:', e);
    }
}

function renderVariables() {
    const tbody = document.getElementById('variables-tbody');
    const table = document.getElementById('variables-table');
    const empty = document.getElementById('variables-empty');
    if (allVariables.length === 0) {
        table.style.display = 'none';
        empty.style.display = '';
        return;
    }
    table.style.display = '';
    empty.style.display = 'none';
    tbody.innerHTML = allVariables.map(v => {
        const isSecret = v.secret;
        const inputType = isSecret ? 'password' : 'text';
        const badge = isSecret ? ' <span style="font-size:10px;color:var(--accent);background:rgba(62,139,255,0.1);padding:1px 5px;border-radius:8px;margin-left:4px">secret</span>' : '';
        return `<tr>
        <td><code>${esc(v.name)}</code>${badge}</td>
        <td><input type="${inputType}" class="var-edit-value" data-name="${esc(v.name)}" value="${esc(v.value)}" style="width:100%;font-family:var(--font-mono);font-size:12px" ${isSecret ? 'placeholder="••••••••" ' : ''}onchange="updateVariable('${esc(v.name)}', this.value)"></td>
        <td style="white-space:nowrap;color:var(--text-muted);font-size:12px">${fmtDate(v.updated_at)}</td>
        <td><button class="btn btn-ghost btn-sm" style="color:var(--danger)" onclick="deleteVariable('${esc(v.name)}')">Delete</button></td>
    </tr>`;
    }).join('');
}

function showAddVariableForm() {
    document.getElementById('add-variable-form').style.display = '';
    document.getElementById('new-var-name').value = '';
    document.getElementById('new-var-value').value = '';
    document.getElementById('new-var-secret').checked = false;
    document.getElementById('new-var-name').focus();
}

function hideAddVariableForm() {
    document.getElementById('add-variable-form').style.display = 'none';
}

async function createVariable() {
    const name = document.getElementById('new-var-name').value.trim();
    const value = document.getElementById('new-var-value').value;
    const secret = document.getElementById('new-var-secret').checked;
    if (!name) return;
    if (!/^[A-Za-z0-9_]+$/.test(name)) {
        toast('Variable name must contain only letters, numbers, and underscores.', 'error');
        return;
    }
    try {
        await api('POST', '/api/variables', { name, value, secret });
        hideAddVariableForm();
        fetchVariables();
    } catch (e) {
        toast('Error: ' + e.message, 'error');
    }
}

async function updateVariable(name, value) {
    try {
        await api('PUT', '/api/variables/' + encodeURIComponent(name), { value });
        fetchVariables();
    } catch (e) {
        toast('Error: ' + e.message, 'error');
    }
}

async function deleteVariable(name) {
    if (!confirm('Delete variable "' + name + '"?')) return;
    try {
        await api('DELETE', '/api/variables/' + encodeURIComponent(name));
        fetchVariables();
    } catch (e) {
        toast('Error: ' + e.message, 'error');
    }
}

