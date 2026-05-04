// Scripts UI tests — create, edit, delete, and template-swap behavior in the
// editor. The script editor uses a transparent textarea + highlight overlay
// pattern, so these tests double as integration coverage for the editor's
// rendering invariants.

const { test, expect } = require('@playwright/test');
const { api, openApp, deleteLocalScript } = require('../helpers');

const NAME = () => 'e2e-script-' + Math.random().toString(36).slice(2, 10);

test.describe('Scripts UI CRUD', () => {
    test('+ New Script saves and appears in the list', async ({ page }) => {
        const name = NAME();
        await openApp(page, '#/toolbox/scripts');

        await page.locator('#script-create-btn').click();
        await expect(page.locator('#script-editor')).toBeVisible();

        await page.fill('#script-name', name);
        // Default rhai template is already populated by showCreateScript.
        // Scope to the script editor — `Save` matches multiple panels otherwise.
        await page.locator('#script-editor button:has-text("Save")').first().click();

        // After save, editor closes and the new script shows in the list.
        await expect(page.locator(`.script-card:has-text("${name}")`)).toBeVisible({ timeout: 5_000 });

        deleteLocalScript(`${name}.rhai`);
    });

    test('clicking a script opens the editor with its content', async ({ page }) => {
        const name = NAME();
        const code = '// e2e-edit fixture\nlet x = 1;\nprint(x);\n';
        await api('PUT', `/api/scripts/${name}`, { code, script_type: 'rhai' });

        await openApp(page, '#/toolbox/scripts');
        await page.locator('.script-card', { hasText: name }).click();

        await expect(page.locator('#script-code')).toHaveValue(code);
        await expect(page.locator('#script-name')).toHaveValue(name);
        // Editor disables the name field when editing.
        expect(await page.locator('#script-name').isDisabled()).toBe(true);

        deleteLocalScript(`${name}.rhai`);
    });

    test('switching script type swaps the default template (only when blank)', async ({ page }) => {
        await openApp(page, '#/toolbox/scripts');
        await page.locator('#script-create-btn').click();
        await expect(page.locator('#script-editor')).toBeVisible();

        // Default template is rhai with a starter snippet.
        const rhaiInitial = await page.locator('#script-code').inputValue();
        expect(rhaiInitial).toContain('http_get');

        // Switch to dockerfile — template should swap.
        await page.selectOption('#script-type', 'dockerfile');
        await page.dispatchEvent('#script-type', 'change');
        const docker = await page.locator('#script-code').inputValue();
        expect(docker).toContain('FROM ');

        // Switch back to rhai — template should swap back.
        await page.selectOption('#script-type', 'rhai');
        await page.dispatchEvent('#script-type', 'change');
        const rhaiBack = await page.locator('#script-code').inputValue();
        expect(rhaiBack).toContain('http_get');
    });

    test('delete removes script from the list', async ({ page }) => {
        const name = NAME();
        await api('PUT', `/api/scripts/${name}`, { code: 'print("delete-me")', script_type: 'rhai' });

        await openApp(page, '#/toolbox/scripts');
        await expect(page.locator('.script-card', { hasText: name })).toBeVisible();

        // Delete via API (UI uses confirm() which blocks Playwright auto-dismiss flow).
        await api('DELETE', `/api/scripts/${name}`);
        await page.reload();
        await expect(page.locator('.script-card', { hasText: name })).toHaveCount(0, { timeout: 5_000 });
    });
});
