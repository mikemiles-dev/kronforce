// Kronforce — First-time user product tour
// Shows a spotlight overlay highlighting key navigation elements

const TOUR_STEPS = [
    {
        target: '#tab-dashboard',
        title: 'Dashboard',
        text: 'Your home base. See job status at a glance, execution charts, recent activity, and pipeline overview.',
        position: 'right'
    },
    {
        target: '#tab-monitor',
        title: 'Monitor',
        text: 'Watch everything in one place. Switch between Jobs (status and management), Runs (execution history), and Events (system log). Filter, search, and drill into details.',
        position: 'right'
    },
    {
        target: '#tab-pipelines',
        title: 'Pipelines',
        text: 'Visualize your pipeline groups. See dependency chains in the Stages view, explore the full dependency Map, set pipeline schedules, and view run history.',
        position: 'right'
    },
    {
        target: '#tab-designer',
        title: 'Designer',
        text: 'Create and edit jobs with a full-page editor. Describe what you want in plain English and let AI fill in the form, or configure every detail manually.',
        position: 'right'
    },
    {
        target: '#tab-toolbox',
        title: 'Toolbox',
        text: 'Shared resources: Scripts (Rhai and Dockerfiles), Variables (global key-value store with secrets), and Connections (encrypted credential profiles for 14 protocol types).',
        position: 'right'
    },
    {
        target: '#tab-settings',
        title: 'Settings',
        text: 'API keys, agents, notification channels, data retention, OIDC/SSO, and system configuration.',
        position: 'right'
    },
    {
        target: '.sidebar-help-btn',
        title: 'Documentation',
        text: 'Searchable in-app reference for task types, API, scripting, connections, migration guides, and more.',
        position: 'right'
    },
    {
        target: '.main-content',
        title: 'Ready to go!',
        text: 'Head to the Designer to create your first job, or explore the Monitor to see what\'s running. You can replay this tour anytime from Settings.',
        position: 'center',
        demoText: 'This is a read-only demo for display purposes only — explore freely, nothing will break. All the data you see is sample data showcasing Kronforce features. To run your own instance, visit kronforce.dev.',
        demoTitle: 'Welcome to the Demo!',
        demoPosition: 'center',
        demoTarget: '.main-content'
    }
];

let tourStep = 0;
let tourOverlay = null;
let tourSteps = [];

function startTour() {
    tourStep = 0;
    tourSteps = TOUR_STEPS.slice();

    // In demo mode, add an intro step and swap the final step text
    const isDemo = typeof currentUser !== 'undefined' && currentUser && currentUser.auth_type === 'demo';
    if (isDemo) {
        tourSteps.unshift({
            target: '.main-content',
            title: 'Welcome to the Kronforce Demo',
            text: 'This is a read-only demo for display purposes only. All the data you see is sample data showcasing Kronforce features. Explore freely — nothing you click will break anything.',
            position: 'center'
        });
        // Swap the last step to demo-specific text and positioning
        const last = tourSteps[tourSteps.length - 1];
        if (last.demoTitle) last.title = last.demoTitle;
        if (last.demoText) last.text = last.demoText;
        if (last.demoPosition) last.position = last.demoPosition;
        if (last.demoTarget) last.target = last.demoTarget;
        delete last.onFinish; // Don't navigate to guide in demo mode
    }

    showTourStep();
}

function showTourStep() {
    removeTourOverlay();

    if (tourStep >= tourSteps.length) {
        localStorage.setItem('kf-tour-done', '1');
        // Show demo banner after tour (not during)
        if (typeof showDemoBanner === 'function') showDemoBanner();
        // Run the last step's onFinish callback (e.g., navigate to Getting Started)
        const lastStep = tourSteps[tourSteps.length - 1];
        if (lastStep && typeof lastStep.onFinish === 'function') lastStep.onFinish();
        return;
    }

    const step = tourSteps[tourStep];
    const target = document.querySelector(step.target);

    // Create overlay
    tourOverlay = document.createElement('div');
    tourOverlay.id = 'tour-overlay';
    tourOverlay.style.cssText = 'position:fixed;inset:0;z-index:10000;pointer-events:auto';

    // Backdrop (4 dark panels around the spotlight)
    const backdrop = document.createElement('div');
    backdrop.style.cssText = 'position:absolute;inset:0;background:rgba(0,0,0,0.7);transition:all 0.3s ease';
    backdrop.onclick = function(e) { if (e.target === backdrop) advanceTour(); };
    tourOverlay.appendChild(backdrop);

    // Spotlight cutout and tooltip
    let spotRect = null;
    if (target && step.position !== 'center') {
        const rect = target.getBoundingClientRect();
        const pad = 6;
        spotRect = {
            x: rect.left - pad,
            y: rect.top - pad,
            w: rect.width + pad * 2,
            h: rect.height + pad * 2
        };

        // SVG mask for spotlight cutout
        backdrop.style.background = 'none';
        const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
        svg.setAttribute('width', '100%');
        svg.setAttribute('height', '100%');
        svg.style.cssText = 'position:absolute;inset:0';
        svg.innerHTML = '<defs><mask id="tour-mask">' +
            '<rect width="100%" height="100%" fill="white"/>' +
            '<rect x="' + spotRect.x + '" y="' + spotRect.y + '" width="' + spotRect.w + '" height="' + spotRect.h + '" rx="8" fill="black"/>' +
            '</mask></defs>' +
            '<rect width="100%" height="100%" fill="rgba(0,0,0,0.75)" mask="url(#tour-mask)"/>';
        tourOverlay.insertBefore(svg, backdrop);
        backdrop.style.display = 'none';

        // Highlight ring
        const ring = document.createElement('div');
        ring.style.cssText = 'position:absolute;border:2px solid var(--accent, #3e8bff);border-radius:8px;pointer-events:none;box-shadow:0 0 0 4px rgba(62,139,255,0.3);transition:all 0.3s ease;' +
            'left:' + spotRect.x + 'px;top:' + spotRect.y + 'px;width:' + spotRect.w + 'px;height:' + spotRect.h + 'px';
        tourOverlay.appendChild(ring);
    }

    // Tooltip card
    const tooltip = document.createElement('div');
    tooltip.style.cssText = 'position:absolute;background:var(--bg-secondary, #161b22);border:1px solid var(--border, #30363d);border-radius:10px;padding:16px 20px;max-width:300px;box-shadow:0 8px 32px rgba(0,0,0,0.5);pointer-events:auto;z-index:10001';

    // Position tooltip — detect mobile (sidebar is horizontal when viewport < 700px)
    const isMobile = window.innerWidth < 700;
    const pos = (step.position === 'center' || !spotRect) ? 'center' : (isMobile ? 'below' : step.position);

    if (pos === 'center') {
        tooltip.style.top = '50%';
        tooltip.style.left = '50%';
        tooltip.style.transform = 'translate(-50%, -50%)';
    } else if (pos === 'below') {
        // Mobile: position below the element, centered horizontally
        const tooltipWidth = 280;
        let left = Math.max(8, spotRect.x + spotRect.w / 2 - tooltipWidth / 2);
        left = Math.min(left, window.innerWidth - tooltipWidth - 8);
        tooltip.style.left = left + 'px';
        tooltip.style.top = (spotRect.y + spotRect.h + 12) + 'px';
        tooltip.style.maxWidth = (window.innerWidth - 16) + 'px';
    } else if (pos === 'right') {
        let tooltipLeft = spotRect.x + spotRect.w + 16;
        let tooltipTop = Math.max(8, spotRect.y + spotRect.h / 2 - 50);
        // Clamp to viewport
        if (tooltipLeft + 300 > window.innerWidth) {
            tooltipLeft = Math.max(8, spotRect.x - 316);
        }
        tooltipTop = Math.min(tooltipTop, window.innerHeight - 250);
        tooltip.style.left = tooltipLeft + 'px';
        tooltip.style.top = tooltipTop + 'px';
    } else if (pos === 'bottom') {
        tooltip.style.left = spotRect.x + 'px';
        tooltip.style.top = (spotRect.y + spotRect.h + 12) + 'px';
    }

    // Arrow for side-positioned tooltips (desktop only)
    if (pos === 'right' && spotRect) {
        const arrow = document.createElement('div');
        arrow.style.cssText = 'position:absolute;left:-8px;top:20px;width:0;height:0;border-top:8px solid transparent;border-bottom:8px solid transparent;border-right:8px solid var(--border, #30363d)';
        tooltip.appendChild(arrow);
        const arrowInner = document.createElement('div');
        arrowInner.style.cssText = 'position:absolute;left:-6px;top:21px;width:0;height:0;border-top:7px solid transparent;border-bottom:7px solid transparent;border-right:7px solid var(--bg-secondary, #161b22)';
        tooltip.appendChild(arrowInner);
    }

    // Step counter
    const counter = document.createElement('div');
    counter.style.cssText = 'font-size:11px;color:var(--text-muted, #8b949e);margin-bottom:6px';
    counter.textContent = (tourStep + 1) + ' of ' + tourSteps.length;
    tooltip.appendChild(counter);

    // Title
    const title = document.createElement('div');
    title.style.cssText = 'font-size:15px;font-weight:600;color:var(--text-primary, #e6edf3);margin-bottom:6px';
    title.textContent = step.title;
    tooltip.appendChild(title);

    // Description
    const desc = document.createElement('div');
    desc.style.cssText = 'font-size:13px;color:var(--text-secondary, #8b949e);line-height:1.5;margin-bottom:14px';
    desc.textContent = step.text;
    tooltip.appendChild(desc);

    // Buttons
    const btns = document.createElement('div');
    btns.style.cssText = 'display:flex;gap:8px;justify-content:flex-end';

    if (tourStep > 0) {
        const back = document.createElement('button');
        back.className = 'btn btn-ghost btn-sm';
        back.textContent = 'Back';
        back.onclick = function() { tourStep--; showTourStep(); };
        btns.appendChild(back);
    }

    const skip = document.createElement('button');
    skip.className = 'btn btn-ghost btn-sm';
    skip.textContent = 'Skip tour';
    skip.style.color = 'var(--text-muted, #8b949e)';
    skip.onclick = function() { removeTourOverlay(); localStorage.setItem('kf-tour-done', '1'); if (typeof showDemoBanner === 'function') showDemoBanner(); };
    btns.appendChild(skip);

    const next = document.createElement('button');
    next.className = 'btn btn-primary btn-sm';
    next.textContent = tourStep === tourSteps.length - 1 ? 'Finish' : 'Next';
    next.onclick = advanceTour;
    btns.appendChild(next);

    tooltip.appendChild(btns);
    tourOverlay.appendChild(tooltip);

    // Keyboard nav
    tourOverlay.tabIndex = 0;
    tourOverlay.onkeydown = function(e) {
        if (e.key === 'Escape') { removeTourOverlay(); localStorage.setItem('kf-tour-done', '1'); if (typeof showDemoBanner === 'function') showDemoBanner(); }
        else if (e.key === 'ArrowRight' || e.key === 'Enter') advanceTour();
        else if (e.key === 'ArrowLeft' && tourStep > 0) { tourStep--; showTourStep(); }
    };

    document.body.appendChild(tourOverlay);
    tourOverlay.focus();
}

function advanceTour() {
    tourStep++;
    showTourStep();
}

function removeTourOverlay() {
    const existing = document.getElementById('tour-overlay');
    if (existing) existing.remove();
    tourOverlay = null;
}

// Auto-start on first visit (after app loads)
function maybeStartTour() {
    if (!localStorage.getItem('kf-tour-done')) {
        setTimeout(startTour, 800);
    } else {
        // Tour already done — show demo banner for returning visitors
        if (typeof showDemoBanner === 'function') showDemoBanner();
    }
}
