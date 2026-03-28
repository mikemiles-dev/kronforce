// Kronforce - Donut chart component
// --- Donut Charts ---

const CHART_COLORS = [
    'var(--success)',
    'var(--danger)',
    'var(--warning)',
    'var(--info)',
    'var(--accent)',
    'var(--text-muted)',
];

const MAX_SEGMENTS = 6;

function renderDonutChart(containerId, data, title) {
    const container = document.getElementById(containerId);
    if (!container) return;

    // Convert object to sorted array of [label, value]
    let entries = Object.entries(data || {}).filter(([, v]) => v > 0);

    if (entries.length === 0) {
        container.innerHTML =
            '<div class="donut-empty">No data</div>';
        return;
    }

    // Sort descending by value
    entries.sort((a, b) => b[1] - a[1]);

    // Cap at MAX_SEGMENTS: top 5 + "Other"
    if (entries.length > MAX_SEGMENTS) {
        const top = entries.slice(0, MAX_SEGMENTS - 1);
        const otherSum = entries.slice(MAX_SEGMENTS - 1).reduce((s, e) => s + e[1], 0);
        entries = [...top, ['Other', otherSum]];
    }

    const total = entries.reduce((s, e) => s + e[1], 0);
    const size = 120;
    const cx = size / 2;
    const cy = size / 2;
    const radius = 44;
    const strokeWidth = 16;
    const circumference = 2 * Math.PI * radius;

    // Build SVG circle segments
    let segments = '';
    let offset = 0;
    entries.forEach(([, value], i) => {
        const pct = value / total;
        const dash = pct * circumference;
        const gap = circumference - dash;
        const color = CHART_COLORS[i % CHART_COLORS.length];
        segments += '<circle cx="' + cx + '" cy="' + cy + '" r="' + radius + '" ' +
            'fill="none" stroke="' + color + '" stroke-width="' + strokeWidth + '" ' +
            'stroke-dasharray="' + dash.toFixed(2) + ' ' + gap.toFixed(2) + '" ' +
            'stroke-dashoffset="' + (-offset).toFixed(2) + '" ' +
            'transform="rotate(-90 ' + cx + ' ' + cy + ')" />';
        offset += dash;
    });

    // Center label
    const centerLabel = '<text x="' + cx + '" y="' + cy + '" text-anchor="middle" ' +
        'dominant-baseline="central" fill="var(--text-primary)" ' +
        'font-size="20" font-weight="700">' + total + '</text>';

    // Legend
    let legend = '<div class="donut-legend">';
    entries.forEach(([label, value], i) => {
        const color = CHART_COLORS[i % CHART_COLORS.length];
        const pct = total > 0 ? Math.round((value / total) * 100) : 0;
        legend += '<div class="donut-legend-item">' +
            '<span class="donut-legend-dot" style="background:' + color + '"></span>' +
            '<span class="donut-legend-label">' + esc(label) + '</span>' +
            '<span class="donut-legend-value">' + value + ' (' + pct + '%)</span>' +
            '</div>';
    });
    legend += '</div>';

    container.innerHTML =
        '<div class="donut-chart">' +
        '<svg width="' + size + '" height="' + size + '" viewBox="0 0 ' + size + ' ' + size + '">' +
        segments + centerLabel +
        '</svg>' +
        legend +
        '</div>';
}
