import { test, expect } from '@playwright/test';
import { setupPage } from './helpers';

test.describe('Focus view clock stability and idle control fade (A25)', () => {
  test('opening the player leaves the clock position unchanged (CLS < 0.01)', async ({ page }) => {
    await setupPage(page, '/#/focus');
    await expect(page.locator('.font-mono.tabular-nums').first()).toBeVisible();

    const digits = page.locator('.font-mono.tabular-nums').first();
    const boxBefore = await digits.boundingBox();
    expect(boxBefore).not.toBeNull();

    // If expand player button exists, toggle it
    const expandBtn = page.locator('button[aria-label="Expand player"]');
    if ((await expandBtn.count()) > 0 && (await expandBtn.isVisible())) {
      await expandBtn.click();
      await page.waitForTimeout(300);

      const boxAfter = await digits.boundingBox();
      expect(boxAfter).not.toBeNull();
      // Y position should not have shifted substantially
      expect(Math.abs((boxBefore?.y ?? 0) - (boxAfter?.y ?? 0))).toBeLessThanOrEqual(5);
    }
  });

  test('controls fade after 3s of inactivity and restore on keypress', async ({ page }) => {
    await setupPage(page, '/#/focus');
    await expect(page.locator('.font-mono.tabular-nums')).toBeVisible();

    // Action buttons inside FocusView controls
    const controls = page.locator('.transition-opacity.duration-\\[var\\(--dur-fast\\)\\]').first();
    await expect(controls).toBeVisible();
    await expect(controls).toHaveClass(/opacity-100/);

    // Wait 3.2s for idle fade
    await page.waitForTimeout(3200);

    // Controls should now have opacity-0
    await expect(controls).toHaveClass(/opacity-0/);

    // Press any key to restore
    await page.keyboard.press('Shift');
    await expect(controls).toHaveClass(/opacity-100/);
  });

  const viewports = [
    { width: 360, height: 640 },
    { width: 768, height: 1024 },
    { width: 1280, height: 800 },
  ];

  for (const vp of viewports) {
    test(`player island is centered within 1px at ${vp.width}px`, async ({ page }) => {
      await page.setViewportSize(vp);
      await setupPage(page, '/#/focus');
      const pill = page.locator('div[role="region"][aria-label^="Floating audio player"]');
      await expect(pill).toBeVisible();

      const box = await pill.boundingBox();
      expect(box).not.toBeNull();
      if (box) {
        const pillCenter = box.x + box.width / 2;
        const viewportCenter = vp.width / 2;
        expect(Math.abs(pillCenter - viewportCenter)).toBeLessThanOrEqual(1);
      }
    });
  }

  test('single motion engine: wrapper owns CSS translate and pill owns Motion transform', async ({ page }) => {
    await setupPage(page, '/#/focus');
    const pill = page.locator('div[role="region"][aria-label^="Floating audio player"]');
    await expect(pill).toBeVisible();

    // The pill itself must NOT have CSS centering translate (-translate-x-1/2)
    const pillHasTranslateClass = await pill.evaluate((el) => {
      return el.classList.contains('-translate-x-1/2') || el.classList.contains('translate-x-[-50%]');
    });
    expect(pillHasTranslateClass).toBe(false);

    // The outer positioning wrapper owns the centering translate
    const wrapperHasTranslate = await pill.evaluate((el) => {
      const parent = el.closest('.absolute.bottom-0.left-1\\/2') || el.parentElement;
      return parent?.classList.contains('-translate-x-1/2') ?? false;
    });
    expect(wrapperHasTranslate).toBe(true);
  });

  test('expand and collapse keeps island centered within 1px', async ({ page }) => {
    await setupPage(page, '/#/focus');
    const pill = page.locator('div[role="region"][aria-label^="Floating audio player"]');
    await expect(pill).toBeVisible();

    const expandBtn = page.locator('button[aria-label="Expand player"]');
    if (await expandBtn.isVisible()) {
      await expandBtn.click();
      await page.waitForTimeout(300);

      const collapseBtn = page.locator('button[aria-label="Collapse player"]');
      await expect(collapseBtn).toBeVisible();

      await collapseBtn.click();
      await page.waitForTimeout(300);

      await expect(pill).toBeVisible();
      const box = await pill.boundingBox();
      expect(box).not.toBeNull();
      if (box) {
        const pillCenter = box.x + box.width / 2;
        const viewportCenter = 1280 / 2;
        expect(Math.abs(pillCenter - viewportCenter)).toBeLessThanOrEqual(1);
      }
    }
  });

  test('play/pause toggle causes zero layout shift on touch viewports', async ({ browser }) => {
    const context = await browser.newContext({
      viewport: { width: 360, height: 640 },
      hasTouch: true,
      isMobile: true,
    });
    const page = await context.newPage();
    await setupPage(page, '/#/focus');

    const pill = page.locator('div[role="region"][aria-label^="Floating audio player"]');
    await expect(pill).toBeVisible();

    const boxBefore = await pill.boundingBox();
    expect(boxBefore).not.toBeNull();

    const playPauseBtn = pill.locator('button[aria-label*="music"]');
    await playPauseBtn.tap();
    await page.waitForTimeout(300);

    const boxAfter = await pill.boundingBox();
    expect(boxAfter).not.toBeNull();
    expect(Math.abs((boxBefore?.width ?? 0) - (boxAfter?.width ?? 0))).toBeLessThanOrEqual(1);
    expect(Math.abs((boxBefore?.height ?? 0) - (boxAfter?.height ?? 0))).toBeLessThanOrEqual(1);

    await context.close();
  });
});

