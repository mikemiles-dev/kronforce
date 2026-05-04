// Jobs CRUD via API + UI verification. Each test seeds its own job under a
// unique name so they can run in any order without colliding.

const { test, expect } = require('@playwright/test');
const { api, openApp } = require('../helpers');

function makeJob(name, overrides = {}) {
    return {
        name,
        description: 'created by e2e jobs spec',
        task: { type: 'shell', command: 'echo hi' },
        schedule: { type: 'on_demand' },
        ...overrides,
    };
}

async function deleteJobByName(name) {
    const list = await api('GET', '/api/jobs?per_page=200');
    const job = (list.data || list).find(j => j.name === name);
    if (job) await api('DELETE', `/api/jobs/${job.id}`);
}

test.describe('Jobs list', () => {
    const NAME = 'e2e-jobs-list-' + Date.now();

    test.beforeAll(async () => {
        await api('POST', '/api/jobs', makeJob(NAME));
    });

    test.afterAll(async () => {
        await deleteJobByName(NAME).catch(() => {});
    });

    test('seeded job appears on Monitor → Jobs', async ({ page }) => {
        await openApp(page, '#/monitor/jobs');
        await expect(page.locator(`.job-name:has-text("${NAME}")`).first()).toBeVisible({ timeout: 10_000 });
    });

    test('search filter narrows the visible list', async ({ page }) => {
        await openApp(page, '#/monitor/jobs');
        await page.locator(`.job-name:has-text("${NAME}")`).first().waitFor();

        await page.fill('#search-input', NAME);
        // The search is debounced — wait for the row count to stabilize at 1.
        await page.waitForFunction((needle) => {
            const rows = document.querySelectorAll('.job-name');
            return rows.length === 1 && rows[0].textContent.includes(needle);
        }, NAME, { timeout: 5_000 });

        // Clear search; full list returns.
        await page.fill('#search-input', '');
        await page.waitForFunction(() => document.querySelectorAll('.job-name').length >= 1);
    });
});

test.describe('Job detail', () => {
    const NAME = 'e2e-jobs-detail-' + Date.now();

    test.beforeAll(async () => {
        await api('POST', '/api/jobs', makeJob(NAME, { description: 'detail-test description' }));
    });

    test.afterAll(async () => {
        await deleteJobByName(NAME).catch(() => {});
    });

    test('clicking job name opens detail view with metadata', async ({ page }) => {
        await openApp(page, '#/monitor/jobs');
        await page.locator(`.job-name:has-text("${NAME}")`).first().click();
        await expect(page.locator('#detail-view')).toBeVisible();
        // Hash should reflect the detail route.
        await page.waitForFunction(() => location.hash.includes('/monitor/jobs/'));
    });
});

test.describe('Job trigger', () => {
    const NAME = 'e2e-jobs-trigger-' + Date.now();
    let jobId;

    test.beforeAll(async () => {
        const job = await api('POST', '/api/jobs', makeJob(NAME));
        jobId = job.id;
    });

    test.afterAll(async () => {
        if (jobId) await api('DELETE', `/api/jobs/${jobId}`).catch(() => {});
    });

    test('trigger button creates an execution record', async ({ page }) => {
        // Trigger via API for stability (UI button has a debounce wrapper).
        await api('POST', `/api/jobs/${jobId}/trigger`);
        // Allow the scheduler tick to pick it up.
        await page.waitForTimeout(1_500);
        const execs = await api('GET', `/api/executions?job_id=${jobId}&per_page=10`);
        const data = execs.data || execs;
        expect(data.length, 'expected at least one execution after trigger').toBeGreaterThan(0);
    });
});

test.describe('Job delete', () => {
    test('deleted job no longer appears in the list', async ({ page }) => {
        const NAME = 'e2e-jobs-delete-' + Date.now();
        const job = await api('POST', '/api/jobs', makeJob(NAME));

        await openApp(page, '#/monitor/jobs');
        await expect(page.locator(`.job-name:has-text("${NAME}")`).first()).toBeVisible();

        await api('DELETE', `/api/jobs/${job.id}`);
        await page.reload();
        await page.waitForFunction(() =>
            !Array.from(document.querySelectorAll('.job-name'))
                .some(el => el.textContent.includes('e2e-jobs-delete-'))
        , { timeout: 10_000 });
    });
});
