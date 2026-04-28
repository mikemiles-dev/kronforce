// Kronforce - Script editor
// --- Scripts ---

let allScripts = [];
let editingScript = null;

async function fetchScripts() {
    try {
        allScripts = await api('GET', '/api/scripts');
        renderScriptsList();
    } catch (e) {
        console.error('fetchScripts:', e);
    }
}

let currentScriptType = 'rhai';

function renderScriptsList() {
    const wrap = document.getElementById('scripts-list-wrap');
    if (allScripts.length === 0) {
        wrap.innerHTML = emptyState('No scripts yet', { sub: 'Scripts are saved as .rhai or .dockerfile files in the scripts directory.' });
        return;
    }
    let html = '<div class="scripts-grid">';
    for (const s of allScripts) {
        const icon = s.script_type === 'dockerfile' ? '&#128051;' : '&#128220;';
        const typeBadge = s.script_type === 'dockerfile' ? ' <span class="badge badge-running">dockerfile</span>' : ' <span class="badge badge-enabled">rhai</span>';
        html += '<div class="script-card" onclick="editScript(\'' + esc(s.name) + '\')">';
        html += '<div class="script-card-header">';
        html += '<span class="script-card-name">' + icon + ' ' + esc(s.name) + typeBadge + '</span>';
        if (canWrite()) html += '<button class="btn btn-ghost btn-sm unpair-btn" onclick="event.stopPropagation();deleteScriptUI(\'' + esc(s.name) + '\')">Delete</button>';
        html += '</div>';
        html += '<div class="script-card-meta">' + s.size + ' bytes' + (s.modified ? ' \u2022 ' + fmtDate(s.modified) : '') + '</div>';
        html += '</div>';
    }
    html += '</div>';
    wrap.innerHTML = html;
}

function showCreateScript() {
    editingScript = null;
    currentScriptType = 'rhai';
    document.getElementById('script-editor-title').textContent = 'New Script';
    document.getElementById('script-name').value = '';
    document.getElementById('script-name').disabled = false;
    document.getElementById('script-name-group').style.display = '';
    document.getElementById('script-type').value = 'rhai';
    document.getElementById('script-type').disabled = false;
    document.getElementById('script-code').value = '// Your script here\nlet resp = http_get("https://example.com/health");\nprint("Status: " + resp.status);\nif resp.status != 200 {\n    fail("Health check failed");\n}';
    document.getElementById('script-editor').style.display = '';
    document.getElementById('scripts-list-wrap').style.display = 'none';
    onScriptTypeChange();
    highlightScript();
}

var rhaiTemplate = '// Your script here\nlet resp = http_get("https://example.com/health");\nprint("Status: " + resp.status);\nif resp.status != 200 {\n    fail("Health check failed");\n}';
var dockerfileTemplate = 'FROM alpine:latest\n\n# This Dockerfile builds and runs successfully out of the box.\n# Edit it to fit your needs.\n\nRUN echo "Hello from Kronforce Docker Build"\n\nCMD ["echo", "Build complete"]';

function onScriptTypeChange() {
    currentScriptType = document.getElementById('script-type').value;
    // Swap template if creating a new script and code matches the other template
    if (!editingScript) {
        var code = document.getElementById('script-code').value;
        if (currentScriptType === 'dockerfile' && (code === rhaiTemplate || !code.trim())) {
            document.getElementById('script-code').value = dockerfileTemplate;
        } else if (currentScriptType === 'rhai' && (code === dockerfileTemplate || !code.trim())) {
            document.getElementById('script-code').value = rhaiTemplate;
        }
    }
    document.getElementById('ref-rhai').style.display = currentScriptType === 'rhai' ? '' : 'none';
    document.getElementById('ref-dockerfile').style.display = currentScriptType === 'dockerfile' ? '' : 'none';
    highlightScript();
}

async function editScript(name) {
    try {
        const script = await api('GET', '/api/scripts/' + encodeURIComponent(name));
        editingScript = name;
        currentScriptType = script.script_type || 'rhai';
        document.getElementById('script-editor-title').textContent = 'Edit: ' + name;
        document.getElementById('script-name').value = name;
        document.getElementById('script-name').disabled = true;
        document.getElementById('script-type').value = currentScriptType;
        document.getElementById('script-type').disabled = true;
        document.getElementById('script-code').value = script.code;
        document.getElementById('script-editor').style.display = '';
        document.getElementById('scripts-list-wrap').style.display = 'none';
        onScriptTypeChange();
        highlightScript();
    } catch (e) {
        toast(e.message, 'error');
    }
}

function closeScriptEditor() {
    document.getElementById('script-editor').style.display = 'none';
    document.getElementById('scripts-list-wrap').style.display = '';
    editingScript = null;
}

async function saveCurrentScript() {
    const name = editingScript || document.getElementById('script-name').value.trim();
    const code = document.getElementById('script-code').value;
    if (!name) { toast('Script name is required', 'error'); return; }
    if (!code.trim()) { toast('Script code is required', 'error'); return; }
    try {
        await api('PUT', '/api/scripts/' + encodeURIComponent(name), { code, script_type: currentScriptType });
        toast('Script "' + name + '" saved');
        closeScriptEditor();
        fetchScripts();
    } catch (e) {
        toast(e.message, 'error');
    }
}

async function deleteScriptUI(name) {
    if (!confirm('Delete script "' + name + '"?')) return;
    try {
        await api('DELETE', '/api/scripts/' + encodeURIComponent(name));
        toast('Script deleted');
        fetchScripts();
    } catch (e) {
        toast(e.message, 'error');
    }
}

async function populateScriptDropdown(selected) {
    const select = document.getElementById('f-script-name');
    try {
        const scripts = await api('GET', '/api/scripts');
        select.innerHTML = '<option value="">Select a script...</option>';
        for (const s of scripts) {
            const sel = s.name === selected ? ' selected' : '';
            select.innerHTML += '<option value="' + esc(s.name) + '"' + sel + '>' + esc(s.name) + '</option>';
        }
    } catch (e) {
        select.innerHTML = '<option value="">Failed to load scripts</option>';
    }
}

async function populateDockerScriptDropdown(selected) {
    const select = document.getElementById('f-docker-script');
    try {
        const scripts = await api('GET', '/api/scripts');
        select.innerHTML = '<option value="">Select a Dockerfile script...</option>';
        for (const s of scripts.filter(s => s.script_type === 'dockerfile')) {
            const sel = s.name === selected ? ' selected' : '';
            select.innerHTML += '<option value="' + esc(s.name) + '"' + sel + '>' + esc(s.name) + '</option>';
        }
    } catch (e) {
        select.innerHTML = '<option value="">Failed to load scripts</option>';
    }
}

// Syntax highlighting
function highlightScript() {
    const code = document.getElementById('script-code').value;
    const highlighted = currentScriptType === 'dockerfile' ? syntaxHighlightDockerfile(code) : syntaxHighlight(code);
    document.getElementById('script-highlight').innerHTML = highlighted + '\n';
}

function syncScriptScroll() {
    const ta = document.getElementById('script-code');
    const hl = document.getElementById('script-highlight');
    hl.scrollTop = ta.scrollTop;
    hl.scrollLeft = ta.scrollLeft;
}

function syntaxHighlight(code) {
    // Tokenize then highlight — avoids HTML entity issues
    let result = '';
    let i = 0;
    while (i < code.length) {
        // Comments
        if (code[i] === '/' && code[i+1] === '/') {
            let end = code.indexOf('\n', i);
            if (end === -1) end = code.length;
            result += '<span class="cmt">' + esc(code.slice(i, end)) + '</span>';
            i = end;
            continue;
        }
        // Strings
        if (code[i] === '"') {
            let end = i + 1;
            while (end < code.length && code[end] !== '"') { if (code[end] === '\\') end++; end++; }
            end = Math.min(end + 1, code.length);
            result += '<span class="str">' + esc(code.slice(i, end)) + '</span>';
            i = end;
            continue;
        }
        // Backtick strings
        if (code[i] === '`') {
            let end = code.indexOf('`', i + 1);
            if (end === -1) end = code.length - 1;
            end++;
            result += '<span class="str">' + esc(code.slice(i, end)) + '</span>';
            i = end;
            continue;
        }
        // Words (keywords, functions, numbers)
        if (/[a-zA-Z_]/.test(code[i])) {
            let end = i;
            while (end < code.length && /[a-zA-Z0-9_]/.test(code[end])) end++;
            const word = code.slice(i, end);
            const keywords = ['let','if','else','while','for','in','fn','return','true','false','loop','break','continue','throw','try','catch','switch','is'];
            const fns = ['print','http_get','http_post','shell_exec','env_var','sleep_ms','fail','parse_int','parse_json','to_string','type_of','len','udp_send','tcp_send','udp_send_hex','tcp_send_hex','hex_encode','hex_decode'];
            if (keywords.includes(word)) result += '<span class="kw">' + word + '</span>';
            else if (fns.includes(word)) result += '<span class="fn">' + word + '</span>';
            else result += esc(word);
            i = end;
            continue;
        }
        // Numbers
        if (/[0-9]/.test(code[i])) {
            let end = i;
            while (end < code.length && /[0-9.]/.test(code[end])) end++;
            result += '<span class="num">' + esc(code.slice(i, end)) + '</span>';
            i = end;
            continue;
        }
        // Operators
        if ('!=<>&|'.includes(code[i])) {
            let op = code[i];
            if (i + 1 < code.length && '=&|>'.includes(code[i+1])) { op += code[i+1]; i++; }
            result += '<span class="op">' + esc(op) + '</span>';
            i++;
            continue;
        }
        result += esc(code[i]);
        i++;
    }
    return result;
}

function syntaxHighlightDockerfile(code) {
    const instructions = ['FROM','RUN','CMD','ENTRYPOINT','COPY','ADD','WORKDIR','ENV','EXPOSE','VOLUME','USER','LABEL','ARG','ONBUILD','STOPSIGNAL','HEALTHCHECK','SHELL','MAINTAINER'];
    const lines = code.split('\n');
    let result = '';
    for (let li = 0; li < lines.length; li++) {
        const line = lines[li];
        if (li > 0) result += '\n';

        // Comments
        const trimmed = line.trimStart();
        if (trimmed.startsWith('#')) {
            result += esc(line.slice(0, line.length - trimmed.length)) + '<span class="cmt">' + esc(trimmed) + '</span>';
            continue;
        }

        // Instruction keyword at start of line
        let matched = false;
        for (const inst of instructions) {
            if (trimmed.toUpperCase().startsWith(inst) && (trimmed.length === inst.length || /\s/.test(trimmed[inst.length]))) {
                const leading = line.slice(0, line.length - trimmed.length);
                const rest = trimmed.slice(inst.length);
                result += esc(leading) + '<span class="kw">' + esc(trimmed.slice(0, inst.length)) + '</span>';
                // Highlight strings and variables in the rest
                result += highlightDockerArgs(rest);
                matched = true;
                break;
            }
        }
        if (!matched) {
            result += highlightDockerArgs(line);
        }
    }
    return result;
}

function highlightDockerArgs(text) {
    let result = '';
    let i = 0;
    while (i < text.length) {
        // Strings
        if (text[i] === '"') {
            let end = i + 1;
            while (end < text.length && text[end] !== '"') { if (text[end] === '\\') end++; end++; }
            end = Math.min(end + 1, text.length);
            result += '<span class="str">' + esc(text.slice(i, end)) + '</span>';
            i = end;
            continue;
        }
        if (text[i] === "'") {
            let end = text.indexOf("'", i + 1);
            if (end === -1) end = text.length - 1;
            end++;
            result += '<span class="str">' + esc(text.slice(i, end)) + '</span>';
            i = end;
            continue;
        }
        // Variables $NAME or ${NAME}
        if (text[i] === '$') {
            let end = i + 1;
            if (text[end] === '{') {
                end = text.indexOf('}', end);
                if (end === -1) end = text.length - 1;
                end++;
            } else {
                while (end < text.length && /[A-Za-z0-9_]/.test(text[end])) end++;
            }
            result += '<span class="fn">' + esc(text.slice(i, end)) + '</span>';
            i = end;
            continue;
        }
        // Flags --name
        if (text[i] === '-' && text[i+1] === '-') {
            let end = i + 2;
            while (end < text.length && /[a-zA-Z0-9_-]/.test(text[end])) end++;
            result += '<span class="op">' + esc(text.slice(i, end)) + '</span>';
            i = end;
            continue;
        }
        result += esc(text[i]);
        i++;
    }
    return result;
}

