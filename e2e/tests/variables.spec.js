// Variables UI CRUD tests. Uses the API for delete (UI delete uses
// confirm() which is harder to dialog-handle reliably).

const { test, expect } = require('@playwright/test');
const { api, openApp } = require('../helpers');

const NAME = () => 'E2E_VAR_' + Math.random().toString(36).slice(2, 8).toUpperCase();

test.describe('Variables UI', () => {
    test('add variable form creates and lists a new variable', async ({ page }) => {
        const name = NAME();
        await openApp(page, '#/toolbox/variables');
        await page.locator('#var-create-btn').click();
        await page.fill('#new-var-name', name);
        await page.fill('#new-var-value', 'hello-world');
        await page.locator('#add-variable-form button:has-text("Save")').click();

        // Variable shows in the list.
        await expect(page.locator(`#variables-tbody tr:has-text("${name}")`)).toBeVisible({ timeout: 5_000 });

        await api('DELETE', `/api/variables/${name}`).catch(() => {});
    });

    test('seeded variable shows in the table after navigation', async ({ page }) => {
        const name = NAME();
        await api('POST', '/api/variables', { name, value: 'list-test', secret: false });

        await openApp(page, '#/toolbox/variables');
        const row = page.locator(`#variables-tbody tr:has-text("${name}")`).first();
        await expect(row).toBeVisible();
        // Value lives in an <input> inside the row, not as text content.
        const input = row.locator('input.var-edit-value');
        await expect(input).toHaveValue('list-test');

        await api('DELETE', `/api/variables/${name}`).catch(() => {});
    });

    test('secret variable value is masked in the UI', async ({ page }) => {
        const name = NAME();
        await api('POST', '/api/variables', { name, value: 'super-sensitive', secret: true });

        await openApp(page, '#/toolbox/variables');
        const row = page.locator(`#variables-tbody tr:has-text("${name}")`).first();
        await expect(row).toBeVisible();
        // The literal secret value must not be present anywhere in the row.
        await expect(row).not.toContainText('super-sensitive');

        await api('DELETE', `/api/variables/${name}`).catch(() => {});
    });

    test('search filter narrows the variable list', async ({ page }) => {
        const a = NAME();
        const b = NAME();
        await api('POST', '/api/variables', { name: a, value: '1' });
        await api('POST', '/api/variables', { name: b, value: '2' });

        await openApp(page, '#/toolbox/variables');
        await expect(page.locator(`#variables-tbody tr:has-text("${a}")`)).toBeVisible();
        await expect(page.locator(`#variables-tbody tr:has-text("${b}")`)).toBeVisible();

        await page.fill('#var-search-input', a);
        // Only the one matching row should remain visible.
        await expect(page.locator(`#variables-tbody tr:has-text("${a}")`)).toBeVisible();
        await page.waitForFunction((other) => {
            const row = Array.from(document.querySelectorAll('#variables-tbody tr'))
                .find(tr => tr.textContent.includes(other));
            return !row || row.style.display === 'none';
        }, b, { timeout: 3_000 });

        await api('DELETE', `/api/variables/${a}`).catch(() => {});
        await api('DELETE', `/api/variables/${b}`).catch(() => {});
    });
});
