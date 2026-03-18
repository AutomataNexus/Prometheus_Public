import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright configuration for the Prometheus E2E test suite.
 *
 * Targets the Prometheus server at http://localhost:3030 (Axum + Leptos WASM).
 * Projects cover desktop browsers (Chromium, Firefox, WebKit) and mobile Chrome.
 */
export default defineConfig({
  testDir: './specs',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: [
    ['html', { open: 'never' }],
    ['list'],
    ...(process.env.CI ? [['github' as const]] : []),
  ],

  use: {
    baseURL: 'http://localhost:3030',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
    actionTimeout: 15_000,
    navigationTimeout: 30_000,
  },

  projects: [
    /* ── Setup ─────────────────────────────────────────── */
    {
      name: 'setup',
      testMatch: /global\.setup\.ts/,
    },

    /* ── Desktop browsers ──────────────────────────────── */
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
      },
      dependencies: ['setup'],
    },
    {
      name: 'firefox',
      use: {
        ...devices['Desktop Firefox'],
      },
      dependencies: ['setup'],
    },
    {
      name: 'webkit',
      use: {
        ...devices['Desktop Safari'],
      },
      dependencies: ['setup'],
    },

    /* ── Mobile ────────────────────────────────────────── */
    {
      name: 'mobile-chrome',
      use: {
        ...devices['Pixel 5'],
      },
      dependencies: ['setup'],
    },
  ],

  /* ── Dev server ────────────────────────────────────── */
  webServer: {
    command: 'cargo run --bin prometheus-server',
    url: 'http://localhost:3030/health',
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
    stdout: 'pipe',
    stderr: 'pipe',
  },
});
