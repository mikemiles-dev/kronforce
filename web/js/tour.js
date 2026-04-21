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
        target: '#tab-jobs',
        title: 'Jobs',
        text: 'Create, edit, and manage all your scheduled tasks. Switch between List, Stages (pipeline view), and Map (dependency graph) tabs.',
        position: 'right'
    },
    {
        target: '#tab-executions',
        title: 'Runs',
        text: 'View execution history across all jobs. Filter by status, search output, and click into any run for full stdout/stderr.',
        position: 'right'
    },
    {
        target: '#tab-events',
        title: 'Events',
        text: 'System event log — job triggers, failures, completions, and alerts. Event-triggered jobs react to patterns here.',
        position: 'right'
    },
    {
        target: '#tab-scripts',
        title: 'Scripts',
        text: 'Store reusable Rhai scripts and Dockerfiles. Reference them from jobs by name instead of inlining commands.',
        position: 'right'
    },
    {
        target: '#tab-variables',
        title: 'Variables',
        text: 'Global key-value store. Use {{VAR_NAME}} in any job field. Output extractions can write here automatically. Supports secrets.',
        position: 'right'
    },
    {
        target: '#tab-settings',
        title: 'Settings',
        text: 'API keys, notification channels (Slack, email, PagerDuty), data retention, and OIDC/SSO configuration.',
        position: 'right'
    },
    {
        target: '#tab-docs',
        title: 'Docs',
        text: 'In-app reference for task types, custom agents, scripting, API, migration guides, and more.',
        position: 'right'
    },
    {
        target: '.main-content',
        title: 'Ready to go!',
        text: 'Create your first job with the + button on the Jobs page, or explore the demo data already loaded. You can replay this tour anytime from Settings.',
        position: 'center',
        // Replaced at runtime for demo mode
        demoText: 'This is a read-only demo for display purposes only — explore freely, nothing will break. All the data you see is sample data showcasing Kronforce features. To run your own instance, visit kronforce.dev.',
        demoTitle: 'Welcome to the Demo!'
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
        // Swap the last step to demo-specific text
        const last = tourSteps[tourSteps.length - 1];
        if (last.demoTitle) last.title = last.demoTitle;
        if (last.demoText) last.text = last.demoText;
    }

    showTourStep();
}

function showTourStep() {
    removeTourOverlay();

    if (tourStep >= tourSteps.length) {
        localStorage.setItem('kf-tour-done', '1');
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

    // Position tooltip
    if (step.position === 'center' || !spotRect) {
        tooltip.style.top = '50%';
        tooltip.style.left = '50%';
        tooltip.style.transform = 'translate(-50%, -50%)';
    } else if (step.position === 'right') {
        tooltip.style.left = (spotRect.x + spotRect.w + 16) + 'px';
        tooltip.style.top = Math.max(8, spotRect.y + spotRect.h / 2 - 50) + 'px';
    } else if (step.position === 'bottom') {
        tooltip.style.left = spotRect.x + 'px';
        tooltip.style.top = (spotRect.y + spotRect.h + 12) + 'px';
    }

    // Arrow for side-positioned tooltips
    if (step.position === 'right' && spotRect) {
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
    skip.onclick = function() { removeTourOverlay(); localStorage.setItem('kf-tour-done', '1'); };
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
        if (e.key === 'Escape') { removeTourOverlay(); localStorage.setItem('kf-tour-done', '1'); }
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
    }
}
