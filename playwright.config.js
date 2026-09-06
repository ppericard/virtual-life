import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './tests/browser',
  timeout: 30000,
  globalTimeout: 300000,
  expect: { timeout: 10000 },
  fullyParallel: true,
  workers: 2,
  retries: 0,
  reporter: [['list'], ['html', { open: 'never' }]],
  use: {
    browserName: 'chromium', viewport: { width: 1280, height: 1000 },
    trace: 'retain-on-failure', screenshot: 'only-on-failure',
  },
});
