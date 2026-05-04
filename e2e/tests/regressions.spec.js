// Each test in this file is a regression — it was a real bug we fixed in
// 0.2.1-alpha. If any of these fail, we've broken something we already
// shipped a fix for.

const { test, expect } = require('@playwright/test');
const { api, openApp, writeLocalScript, deleteLocalScript } = require('../helpers');

test.describe('Browser back button (commit b6fed31)', () => {
    test('walks through in-app navigation instead of leaving Kronforce', async ({ page }) => {
        // Land on dashboard, then traverse to monitor → pipelines.
        await openApp(page, '#/dashboard');
        await page.waitForFunction(() => location.hash === '#/dashboard');

        await page.evaluate(() => showPage('monitor'));
        await page.waitForFunction(() => location.hash.startsWith('#/monitor'));

        await page.evaluate(() => showPage('pipelines'));
        await page.waitForFunction(() => location.hash.startsWith('#/pipelines'));

        // Going back should land on monitor (the previous in-app page), not
        // navigate out of the app. With history.replaceState (the bug), back
        // would leave the SPA entirely.
        await page.goBack();
        await page.waitForFunction(() => location.hash.startsWith('#/monitor'));
        const hash = await page.evaluate(() => location.hash);
        expect(hash).toMatch(/^#\/monitor/);
    });
});

test.describe('Settings → Agents card click (commit b6fed31)', () => {
    test('does not blank the page when clicked', async ({ page }) => {
        // Seed one agent so the cards render.
        await api('POST', '/api/agents/register', {
            name: 'e2e-agent',
            tags: ['e2e'],
            hostname: 'localhost',
            address: '127.0.0.1',
            port: 18999,
            agent_type: 'standard',
        });

        await openApp(page, '#/settings');
        await page.evaluate(() => showSettingsTab('agents'));
        await page.locator('#settings-agents-wrap .card').first().waitFor();

        await page.locator('#settings-agents-wrap .card').first().click();

        // The Settings view itself must still be visible. The bug
        // (showPage('agents')) hid every view because 'agents' is not in
        // ALL_VIEWS, leaving a blank page.
        await expect(page.locator('#settings-view')).toBeVisible();
        await expect(page.locator('#settings-panel-agents')).toBeVisible();
    });
});

test.describe('Script editor (commits 5c91096 + f9ff72e)', () => {
    const SCRIPT_NAME = 'e2e-editor-test';
    const SCRIPT_CODE = '// e2e test script\nlet x = 42;\nprint(x);\n';

    test.beforeEach(() => {
        writeLocalScript(`${SCRIPT_NAME}.rhai`, SCRIPT_CODE);
    });

    test.afterEach(() => {
        deleteLocalScript(`${SCRIPT_NAME}.rhai`);
    });

    test('opens with the script code visible in the textarea', async ({ page }) => {
        await openApp(page, '#/toolbox/scripts');
        await page.locator('#scripts-list-wrap .script-card').first().waitFor();

        // Find and click our seeded script.
        await page.locator('.script-card', { hasText: SCRIPT_NAME }).click();

        // Textarea must contain the actual code (regression: it was empty
        // when editScript silently failed to populate it).
        await expect(page.locator('#script-code')).toHaveValue(SCRIPT_CODE);

        // Textarea must be a visible color (not transparent like the original
        // buggy CSS that left users staring at an empty box).
        const ta = page.locator('#script-code');
        const taColor = await ta.evaluate(el => getComputedStyle(el).color);
        expect(taColor).not.toBe('rgba(0, 0, 0, 0)');
    });

    test('renders syntax-highlighted spans on top of the textarea', async ({ page }) => {
        await openApp(page, '#/toolbox/scripts');
        await page.locator('#scripts-list-wrap .script-card').first().waitFor();
        await page.locator('.script-card', { hasText: SCRIPT_NAME }).click();

        // The highlight overlay must contain colored spans for keywords.
        await expect(page.locator('#script-highlight .kw').first()).toBeAttached();

        // Highlight overlay must sit above the textarea so the colored spans
        // are not covered. Regression from f9ff72e: the textarea was
        // z-index 2 above the highlight at z-index 1, hiding all coloring.
        const layering = await page.evaluate(() => ({
            highlightZ: parseInt(getComputedStyle(document.getElementById('script-highlight')).zIndex, 10) || 0,
            textareaZ: parseInt(getComputedStyle(document.getElementById('script-code')).zIndex, 10) || 0,
        }));
        expect(layering.highlightZ).toBeGreaterThan(layering.textareaZ);
    });
});

test.describe('Event chip styling (commits 91ef7ee + later)', () => {
    test('agent and api-key events render with the .event-link affordance', async ({ page }) => {
        // Force at least one event with an api_key_id and agent_id by
        // registering an agent (the controller logs an `agent.registered`
        // event with an agent_id, and the bearer used for the call is the
        // admin api key — so the event row gets api_key chip + agent chip).
        await api('POST', '/api/agents/register', {
            name: 'e2e-chip-agent',
            tags: ['e2e'],
            hostname: 'localhost',
            address: '127.0.0.1',
            port: 18998,
            agent_type: 'standard',
        });

        await openApp(page, '#/monitor/events');

        // Wait for the events list to render at least one row.
        await page.locator('.event-item').first().waitFor({ timeout: 5_000 });

        // The .event-link class must be defined and applied to chips that
        // navigate somewhere (cursor:pointer is the visible affordance).
        const linkChip = page.locator('.event-link').first();
        await expect(linkChip).toBeVisible();
        const cursor = await linkChip.evaluate(el => getComputedStyle(el).cursor);
        expect(cursor).toBe('pointer');
    });
});
