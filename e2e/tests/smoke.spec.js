// Smoke tests — every top-level page renders without errors and sub-tabs
// swap content correctly. These should be fast and break on any major
// nav/routing regression.

const { test, expect } = require('@playwright/test');
const { openApp } = require('../helpers');

test.describe('Page smoke tests', () => {
    const pages = [
        { route: '#/dashboard', visibleId: 'dashboard-view' },
        { route: '#/monitor/jobs', visibleId: 'monitor-view' },
        { route: '#/monitor/runs', visibleId: 'monitor-view' },
        { route: '#/monitor/events', visibleId: 'monitor-view' },
        { route: '#/pipelines/stages', visibleId: 'pipelines-view' },
        { route: '#/pipelines/map', visibleId: 'pipelines-view' },
        { route: '#/designer', visibleId: 'designer-view' },
        { route: '#/toolbox/scripts', visibleId: 'toolbox-view' },
        { route: '#/toolbox/variables', visibleId: 'toolbox-view' },
        { route: '#/toolbox/connections', visibleId: 'toolbox-view' },
        { route: '#/settings', visibleId: 'settings-view' },
        { route: '#/docs', visibleId: 'docs-view' },
    ];

    for (const { route, visibleId } of pages) {
        test(`${route} renders ${visibleId}`, async ({ page }) => {
            const errors = [];
            page.on('pageerror', (err) => errors.push(err.message));
            await openApp(page, route);
            await expect(page.locator(`#${visibleId}`)).toBeVisible();
            expect(errors, `JS errors on ${route}: ${errors.join('; ')}`).toEqual([]);
        });
    }
});

test.describe('Sub-tab navigation', () => {
    test('monitor: jobs ↔ runs ↔ events swaps panels', async ({ page }) => {
        await openApp(page, '#/monitor/jobs');
        await expect(page.locator('#monitor-jobs-panel')).toBeVisible();

        await page.evaluate(() => setSubTab('monitor', 'runs'));
        await expect(page.locator('#monitor-runs-panel')).toBeVisible();
        await expect(page.locator('#monitor-jobs-panel')).toBeHidden();

        await page.evaluate(() => setSubTab('monitor', 'events'));
        await expect(page.locator('#monitor-events-panel')).toBeVisible();
        await expect(page.locator('#monitor-runs-panel')).toBeHidden();
    });

    test('toolbox: scripts ↔ variables ↔ connections swaps panels', async ({ page }) => {
        await openApp(page, '#/toolbox/scripts');
        await expect(page.locator('#toolbox-scripts-panel')).toBeVisible();

        await page.evaluate(() => setSubTab('toolbox', 'variables'));
        await expect(page.locator('#toolbox-variables-panel')).toBeVisible();
        await expect(page.locator('#toolbox-scripts-panel')).toBeHidden();

        await page.evaluate(() => setSubTab('toolbox', 'connections'));
        await expect(page.locator('#toolbox-connections-panel')).toBeVisible();
        await expect(page.locator('#toolbox-variables-panel')).toBeHidden();
    });

    test('pipelines: stages ↔ map swaps panels', async ({ page }) => {
        await openApp(page, '#/pipelines/stages');
        await expect(page.locator('#pipelines-stages-panel')).toBeVisible();
        await page.evaluate(() => setSubTab('pipelines', 'map'));
        await expect(page.locator('#pipelines-map-panel')).toBeVisible();
        await expect(page.locator('#pipelines-stages-panel')).toBeHidden();
    });
});

test.describe('Sidebar nav', () => {
    test('every nav-tab button routes to a visible view', async ({ page }) => {
        await openApp(page, '#/dashboard');
        // Sidebar items are buttons (not anchors) with id="tab-<page>" and
        // onclick="showPage('<page>')".
        const tabIds = await page.locator('.sidebar-nav .nav-tab[id^="tab-"]').evaluateAll(
            els => els.map(el => el.id.replace(/^tab-/, ''))
        );
        expect(tabIds.length).toBeGreaterThan(0);

        for (const page_ of tabIds) {
            await page.locator(`#tab-${page_}`).click();
            // After click, the matching view section must be visible.
            await page.waitForFunction((p) => {
                const el = document.getElementById(`${p}-view`);
                return el && getComputedStyle(el).display !== 'none';
            }, page_, { timeout: 5_000 });
            // Hash should mention this page name.
            await page.waitForFunction((p) => location.hash.includes(p), page_);
        }
    });
});
