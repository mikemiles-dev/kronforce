// Kronforce - Groups page
// --- Groups Page ---

let groupsPageJobs = [];

async function fetchGroupsPage() {
    try {
        const [groups, jobsRes] = await Promise.all([
            api('GET', '/api/jobs/groups'),
            api('GET', '/api/jobs?per_page=1000'),
        ]);
        const jobs = jobsRes.data;
        groupsPageJobs = jobs;

        // Collect jobs per group
        const jobsByGroup = {};
        for (const j of jobs) {
            const g = j.group || 'Default';
            if (!jobsByGroup[g]) jobsByGroup[g] = [];
            jobsByGroup[g].push(j);
        }

        const grid = document.getElementById('groups-grid');
        if (!grid) return;

        if (groups.length === 0 && jobs.length === 0) {
            grid.innerHTML = renderRichEmptyState({
                icon: '&#128193;',
                title: 'No groups yet',
                description: 'Groups help you organize jobs. Create a group, then assign jobs to it from the Jobs page or when creating a job.',
                actions: [
                    { label: 'Create Group', onclick: 'createNewGroup()', primary: true },
                ],
            });
            return;
        }

        let html = '';

        // Group cards
        for (const g of groups) {
            const groupJobs = jobsByGroup[g] || [];
            const count = groupJobs.length;
            const color = groupColor(g);
            html += '<div class="group-card">';
            html += '<div class="group-card-header" style="cursor:pointer" onclick="navToGroupJobs(\'' + esc(g) + '\')">';
            html += '<span class="group-card-dot" style="background:' + color + '"></span>';
            html += '<span class="group-card-name">' + esc(g) + '</span>';
            html += '<span style="margin-left:auto;font-size:11px;color:var(--accent)">&rarr;</span>';
            html += '</div>';
            html += '<div class="group-card-count">' + count + ' job' + (count !== 1 ? 's' : '') + '</div>';
            // Show job names (up to 5)
            if (groupJobs.length > 0) {
                html += '<div class="group-card-jobs">';
                for (const j of groupJobs.slice(0, 5)) {
                    html += '<div class="group-card-job">' + esc(j.name) + '</div>';
                }
                if (groupJobs.length > 5) {
                    html += '<div class="group-card-job" style="color:var(--text-muted)">...and ' + (groupJobs.length - 5) + ' more</div>';
                }
                html += '</div>';
            }
            html += '<div class="group-card-actions">';
            html += '<button class="btn btn-ghost btn-sm" onclick="renameGroup(\'' + esc(g) + '\')">Rename</button>';
            html += '<button class="btn btn-ghost btn-sm" style="color:var(--danger)" onclick="deleteGroup(\'' + esc(g) + '\')">Delete</button>';
            html += '</div>';
            html += '</div>';
        }

        grid.innerHTML = html;
    } catch (e) {
        console.error('fetchGroupsPage:', e);
    }
}

async function createNewGroup() {
    const name = prompt('Enter new group name:');
    if (!name || !name.trim()) return;
    const trimmed = name.trim();
    if (cachedGroups.includes(trimmed)) {
        toast('Group "' + trimmed + '" already exists', 'error');
        return;
    }
    // To create a group, we need at least one job in it.
    // Navigate to Jobs page so they can select and assign.
    // Add the new name to cachedGroups so it shows in the Set Group prompt.
    cachedGroups.push(trimmed);
    cachedGroups.sort();
    toast('Select jobs on the Jobs page, then click "Set Group" and choose "' + trimmed + '"');
    showPage('jobs');
}

function navToGroupJobs(group) {
    groupFilter = group;
    showPage('jobs');
    const sel = document.getElementById('group-filter');
    if (sel) sel.value = group;
    fetchJobs(true);
}

async function renameGroup(oldName) {
    const newName = prompt('Rename group "' + oldName + '" to:', oldName);
    if (!newName || newName === oldName) return;
    try {
        await api('PUT', '/api/jobs/rename-group', { old_name: oldName, new_name: newName });
        toast('Group renamed to "' + newName + '"');
        fetchGroupsPage();
        fetchGroups();
    } catch (e) {
        toast('Error: ' + e.message, 'error');
    }
}

async function deleteGroup(name) {
    if (name === 'Default') {
        toast('The Default group cannot be deleted', 'error');
        return;
    }
    if (!confirm('Delete group "' + name + '"? All jobs in this group will be moved to the Default group.')) return;
    try {
        await api('PUT', '/api/jobs/rename-group', { old_name: name, new_name: 'Default' });
        toast('Group "' + name + '" deleted — jobs moved to Default');
        fetchGroupsPage();
        fetchGroups();
    } catch (e) {
        toast('Error: ' + e.message, 'error');
    }
}
