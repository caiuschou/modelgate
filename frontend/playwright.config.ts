import { defineConfig, devices } from '@playwright/test'

const baseURL = process.env.PLAYWRIGHT_BASE_URL ?? 'http://127.0.0.1:3000'

export default defineConfig({
  testDir: 'e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: 'list',
  globalSetup: './e2e/global-setup.ts',
  timeout: 60_000,
  expect: { timeout: 15_000 },
  use: {
    baseURL,
    storageState: 'playwright/.auth/user.json',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    ...devices['Desktop Chrome'],
  },
  webServer: [
    {
      command: 'node ../e2e/run-modelgate-stack.mjs',
      url: 'http://127.0.0.1:8000/healthz',
      reuseExistingServer: !process.env.CI,
      timeout: 180_000,
    },
    {
      command: 'npm run dev -- --host 127.0.0.1 --port 3000 --strictPort',
      url: baseURL,
      reuseExistingServer: !process.env.CI,
      timeout: 120_000,
    },
  ],
})
