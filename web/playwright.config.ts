import { defineConfig, devices } from '@playwright/test';
import { execSync } from 'child_process';

// Resolve Nix browser path if not explicitly provided
if (!process.env.PLAYWRIGHT_BROWSERS_PATH) {
  try {
    const nixPath = execSync('nix eval --raw nixpkgs#playwright-driver.browsers', { encoding: 'utf-8' }).trim();
    if (nixPath) {
      process.env.PLAYWRIGHT_BROWSERS_PATH = nixPath;
    }
  } catch {
    // Keep environment fallback
  }
}

const E2E_PORT = process.env.VITE_PORT || '5174';

export default defineConfig({
  testDir: './e2e',
  testMatch: '**/*.spec.ts',
  timeout: 30000,
  expect: {
    timeout: 5000,
  },
  fullyParallel: false,
  workers: 1,
  reporter: [['list']],
  outputDir: './.design/e2e/test-results',
  use: {
    baseURL: `http://localhost:${E2E_PORT}`,
    trace: 'retain-on-failure',
    video: 'retain-on-failure',
    screenshot: 'only-on-failure',
    actionTimeout: 5000,
    navigationTimeout: 10000,
  },
  globalSetup: './e2e/global-setup.ts',
  globalTeardown: './e2e/global-teardown.ts',
  webServer: {
    command: `VITE_PORT=${E2E_PORT} npm run dev`,
    url: `http://localhost:${E2E_PORT}`,
    reuseExistingServer: false,
    timeout: 30000,
    env: {
      VITE_PORT: E2E_PORT,
      VITE_API_TARGET: process.env.VITE_API_TARGET || 'http://127.0.0.1:8799',
    },
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
});

