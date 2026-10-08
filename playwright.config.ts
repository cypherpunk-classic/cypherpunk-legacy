import { existsSync } from 'node:fs';
import { defineConfig, devices } from '@playwright/test';

// In this sandbox a Chromium is preinstalled at a known path; in CI Playwright
// installs its own, so only pin executablePath when that path actually exists.
const PINNED = process.env.CHROME_PATH || '/opt/pw-browsers/chromium';
const executablePath = existsSync(PINNED) ? PINNED : undefined;
const args = ['--use-gl=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist', '--autoplay-policy=no-user-gesture-required'];
const launchOptions = executablePath ? { executablePath, args } : { args };

export default defineConfig({
  testDir: 'tests/e2e',
  timeout: 300_000,
  expect: { timeout: 20_000 },
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: [['list']],
  use: {
    baseURL: 'http://localhost:4173',
    viewport: { width: 1280, height: 720 },
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
    launchOptions,
  },
  webServer: {
    command: 'npx vite --port 4173 --strictPort',
    port: 4173,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'], viewport: { width: 1280, height: 720 }, launchOptions } }],
});
