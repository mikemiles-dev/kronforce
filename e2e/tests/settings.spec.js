// Settings page tests — tab switching, retention persistence.

const { test, expect } = require('@playwright/test');
const { api, openApp } = require('../helpers');

test.describe('Settings tabs', () => {
    test('all settings tabs swap content', async ({ page }) => {
        await openApp(page, '#/settings');
        const tabs = ['general', 'agents', 'auth', 'notifications'];
        for (const tab of tabs) {
            await page.evaluate((t) => showSettingsTab(t), tab);
            await expect(page.locator(`#settings-panel-${tab}`)).toBeVisible();
            for (const other of tabs.filter(x => x !== tab)) {
                await expect(page.locator(`#settings-panel-${other}`)).toBeHidden();
            }
        }
    });
});

test.describe('Retention setting', () => {
    test('saved value persists across reload', async ({ page }) => {
        await openApp(page, '#/settings');
        await page.evaluate(() => showSettingsTab('general'));

        // The control is a <select> with onchange="saveRetention()" so
        // selecting a value triggers the save automatically.
        await page.selectOption('#retention-days', '14');
        await expect(page.locator('#retention-status')).toContainText('Saved', { timeout: 3_000 });

        await page.reload();
        await page.evaluate(() => showSettingsTab('general'));
        await expect(page.locator('#retention-days')).toHaveValue('14');

        // Verify via API that the controller persisted it.
        const settings = await api('GET', '/api/settings');
        expect(settings.retention_days).toBe('14');
    });
});

test.describe('Notification settings form', () => {
    test('email enable toggles persist round-trip', async ({ page }) => {
        await openApp(page, '#/settings');
        await page.evaluate(() => showSettingsTab('notifications'));
        const cb = page.locator('#notif-email-enabled');
        await expect(cb).toBeVisible();
        const initial = await cb.isChecked();
        // Flip and save.
        if (initial) await cb.uncheck(); else await cb.check();
        await page.locator('button[onclick="saveNotificationSettings()"]').click();
        await page.reload();
        await page.evaluate(() => showSettingsTab('notifications'));
        const after = await page.locator('#notif-email-enabled').isChecked();
        expect(after).toBe(!initial);
        // Restore.
        if (after) await page.locator('#notif-email-enabled').uncheck();
        else await page.locator('#notif-email-enabled').check();
        await page.locator('button[onclick="saveNotificationSettings()"]').click();
    });
});
