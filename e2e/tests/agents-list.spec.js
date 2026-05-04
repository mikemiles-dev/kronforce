// Agents listing — register an agent over the API and verify it shows up
// under Settings → Agents with the right metadata.

const { test, expect } = require('@playwright/test');
const { api, openApp } = require('../helpers');

test.describe('Agents list (Settings → Agents)', () => {
    test('registered agent shows hostname, address, and tags', async ({ page }) => {
        const name = 'e2e-agent-' + Date.now();
        await api('POST', '/api/agents/register', {
            name,
            tags: ['ci', 'linux'],
            hostname: 'e2e-host.example',
            address: '10.0.0.42',
            port: 19000,
            agent_type: 'standard',
        });

        await openApp(page, '#/settings');
        await page.evaluate(() => showSettingsTab('agents'));

        const card = page.locator(`#settings-agents-wrap .card:has-text("${name}")`);
        await expect(card).toBeVisible({ timeout: 5_000 });
        await expect(card).toContainText('e2e-host.example');
        await expect(card).toContainText('10.0.0.42');
        await expect(card).toContainText('ci');
        await expect(card).toContainText('linux');

        // List endpoint also surfaces it.
        const list = await api('GET', '/api/agents');
        expect(list.find(a => a.name === name)).toBeTruthy();
    });
});
