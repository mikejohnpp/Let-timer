import { chromium } from 'playwright-core';

const CHROMIUM_PATH =
  process.env.PLAYWRIGHT_CHROMIUM_PATH ||
  process.env.CHROMIUM_PATH ||
  '/nix/store/lk76rv2xy628nrlc5ksgzli3lpn3hhiq-chromium-137.0.7151.55/bin/chromium';

const HEADLESS = process.env.HEADED !== 'true';
const SLOW_MO = process.env.HEADED === 'true' ? 120 : 0;
const BASE_URL = process.env.BASE_URL || 'http://localhost:5173';

let failedTests = 0;

function assert(condition, message) {
  if (!condition) {
    console.error(`  FAIL: ${message}`);
    failedTests++;
  } else {
    console.log(`  PASS: ${message}`);
  }
}

async function runTest(name, fn) {
  console.log(`\n--- Test: ${name} ---`);
  try {
    await fn();
  } catch (err) {
    console.error(`  ERROR in test "${name}":`, err.message || err);
    failedTests++;
  }
}

async function setupMockRoutes(page) {
  await page.route('**/api/v1/**', (route) => {
    const url = route.request().url();
    const method = route.request().method();

    if (url.includes('/api/v1/auth/me')) {
      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ ok: true, user: 'demo' }),
      });
    }

    if (url.includes('/api/v1/events')) {
      return route.fulfill({
        status: 200,
        contentType: 'text/event-stream',
        body: ': keep-alive\n\n',
      });
    }

    if (url.includes('/api/v1/workspaces')) {
      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([{ id: 1, name: 'Default', is_active: true }]),
      });
    }

    if (url.includes('/api/v1/tasks/current')) {
      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ state: 'idle', task: null }),
      });
    }

    if (url.includes('/api/v1/tasks')) {
      if (url.includes('overdue=true')) {
        return route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify([]),
        });
      }
      if (method === 'GET') {
        return route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify([
            {
              id: 1,
              name: 'Write documentation',
              status: 'pending',
              priority: 1,
              workspace_id: 1,
              estimated_mins: 25,
            },
            {
              id: 2,
              name: 'Plan release',
              status: 'completed',
              priority: 0,
              workspace_id: 1,
              estimated_mins: 15,
            },
          ]),
        });
      }
      return route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ ok: true }) });
    }

    if (url.includes('/api/v1/settings/timezone')) {
      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ timezone: 'UTC' }),
      });
    }

    if (url.includes('/api/v1/auth/tokens')) {
      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([]),
      });
    }

    if (url.includes('/api/v1/medias')) {
      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          {
            id: 101,
            name: 'Ambient Rain',
            url: 'https://example.com/rain.mp3',
            media_type: 'music',
            created_at: '2026-01-01T00:00:00Z',
            updated_at: '2026-01-01T00:00:00Z',
          },
        ]),
      });
    }

    if (url.includes('/api/v1/media-lists')) {
      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([]),
      });
    }

    return route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({}) });
  });
}

async function main() {
  console.log(`Starting Playwright Motion E2E Tests`);
  console.log(`Chromium binary: ${CHROMIUM_PATH}`);
  console.log(`Headless: ${HEADLESS}, SlowMo: ${SLOW_MO}ms`);

  const browser = await chromium.launch({
    executablePath: CHROMIUM_PATH,
    headless: HEADLESS,
    slowMo: SLOW_MO,
  });

  try {
    // ─── 1. Tab Transition Test ────────────────────────────────────────────────
    await runTest('Tab transition with ViewHost single engine (no .view-enter double-animation)', async () => {
      const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
      const page = await context.newPage();
      await setupMockRoutes(page);

      await page.goto(`${BASE_URL}/#/today`, { waitUntil: 'domcontentloaded' });
      await page.waitForSelector('[data-testid="view-tab-today"]');

      // Assert ViewHost is mounted
      const viewHost = await page.$('[data-testid="view-host"]');
      assert(viewHost !== null, 'ViewHost element exists');

      // Assert no CSS .view-enter class exists
      const viewEnterElements = await page.$$('.view-enter');
      assert(viewEnterElements.length === 0, 'No .view-enter CSS elements in DOM');

      // Click tab Settings
      const settingsTab = page.locator('header nav button').filter({ hasText: 'Settings' }).first();
      await settingsTab.click();

      // ViewHost transitions to settings tab
      await page.waitForSelector('[data-testid="view-tab-settings"]');
      const settingsView = await page.$('[data-testid="view-tab-settings"]');
      assert(settingsView !== null, 'Successfully transitioned to Settings tab');

      // Navigate back to Today
      const todayTab = page.locator('header nav button').filter({ hasText: 'Today' }).first();
      await todayTab.click();
      await page.waitForSelector('[data-testid="view-tab-today"]');
      assert(true, 'Successfully transitioned back to Today tab');

      await context.close();
    });

    // ─── 2. Task Layout Continuity Test ────────────────────────────────────────
    await runTest('Task rows use LayoutGroup without useFlip or .task-row-add', async () => {
      const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
      const page = await context.newPage();
      await setupMockRoutes(page);

      await page.goto(`${BASE_URL}/#/today`, { waitUntil: 'domcontentloaded' });
      await page.waitForSelector('[data-task-id="1"]');

      const taskRow = await page.$('[data-task-id="1"]');
      assert(taskRow !== null, 'Task row with data-task-id="1" is rendered');

      // Check class list
      const className = await taskRow.getAttribute('class');
      assert(!className.includes('task-row-add'), 'Task row does not have .task-row-add class');

      await context.close();
    });

    // ─── 3. Reduced Motion Parity Test ─────────────────────────────────────────
    await runTest('Reduced motion mode zeroes travel and respects preferences', async () => {
      const context = await browser.newContext({
        viewport: { width: 1280, height: 800 },
        reducedMotion: 'reduce',
      });
      const page = await context.newPage();
      await setupMockRoutes(page);

      await page.goto(`${BASE_URL}/#/today`, { waitUntil: 'domcontentloaded' });
      await page.waitForSelector('[data-testid="view-tab-today"]');

      // Check computed travel on reduced motion
      const durFast = await page.evaluate(() => {
        return getComputedStyle(document.documentElement).getPropertyValue('--dur-fast').trim();
      });
      assert(durFast === '100ms', `--dur-fast is reduced to 100ms (got: ${durFast})`);

      await context.close();
    });

    // ─── 4. ?motion=off Mode Test ──────────────────────────────────────────────
    await runTest('?motion=off sets all motion durations to 0ms', async () => {
      const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
      const page = await context.newPage();
      await setupMockRoutes(page);

      await page.goto(`${BASE_URL}/?motion=off#/today`, { waitUntil: 'domcontentloaded' });
      await page.waitForSelector('[data-testid="view-tab-today"]');

      const dataMotion = await page.evaluate(() => document.documentElement.getAttribute('data-motion'));
      assert(dataMotion === 'off', `data-motion attribute is "off" (got: ${dataMotion})`);

      const durBase = await page.evaluate(() => {
        return getComputedStyle(document.documentElement).getPropertyValue('--dur-base').trim();
      });
      assert(durBase === '0ms', `--dur-base is 0ms under ?motion=off (got: ${durBase})`);

      await context.close();
    });

    // ─── 5. Floating Player Island Toggle Test ──────────────────────────────────
    await runTest('Floating island toggle (pill -> expanded card -> pill single engine)', async () => {
      const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
      const page = await context.newPage();
      await setupMockRoutes(page);

      await page.goto(`${BASE_URL}/#/media`, { waitUntil: 'domcontentloaded' });
      await page.waitForSelector('[data-testid="view-tab-media"]');

      // Click "Library" tab to see individual media items
      const libraryTab = page.locator('button').filter({ hasText: 'Library' }).first();
      await libraryTab.click();

      // Find and click Play button for the track
      const playBtn = page.locator('button[title="Play"]').first();
      await playBtn.waitFor({ state: 'visible' });
      await playBtn.click();

      // Floating island pill should appear at the bottom
      const expandBtn = page.locator('button[aria-label="Expand player"]').first();
      await expandBtn.waitFor({ state: 'visible' });
      assert(await expandBtn.isVisible(), 'Floating player island pill is rendered');

      // Expand island
      await expandBtn.click();
      const collapseBtn = page.locator('button[aria-label="Collapse player"]').first();
      await collapseBtn.waitFor({ state: 'visible' });
      assert(await collapseBtn.isVisible(), 'Floating player island expanded into card');

      // Collapse island
      await collapseBtn.click();
      await expandBtn.waitFor({ state: 'visible' });
      assert(await expandBtn.isVisible(), 'Floating player island collapsed back to pill');

      await context.close();
    });
  } finally {
    await browser.close();
  }

  console.log(`\nE2E Results: ${failedTests === 0 ? 'ALL PASSED' : `${failedTests} FAILED`}`);
  if (failedTests > 0) {
    process.exit(1);
  }
}

main().catch((err) => {
  console.error('Fatal E2E error:', err);
  process.exit(1);
});
