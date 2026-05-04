// @ts-check
const { defineConfig, devices } = require('@playwright/test');

const PORT = process.env.KRONFORCE_E2E_PORT || '18080';
const BASE_URL = `http://127.0.0.1:${PORT}`;

module.exports = defineConfig({
    testDir: './tests',
    timeout: 30_000,
    fullyParallel: false, // single shared controller; tests share state
    forbidOnly: !!process.env.CI,
    retries: process.env.CI ? 2 : 0,
    workers: 1, // shared controller — one worker
    reporter: process.env.CI ? [['html'], ['list']] : 'list',
    globalSetup: require.resolve('./global-setup.js'),
    globalTeardown: require.resolve('./global-teardown.js'),
    use: {
        baseURL: BASE_URL,
        trace: 'retain-on-failure',
        screenshot: 'only-on-failure',
        video: 'retain-on-failure',
    },
    projects: [
        {
            name: 'chromium',
            use: { ...devices['Desktop Chrome'] },
        },
    ],
});
