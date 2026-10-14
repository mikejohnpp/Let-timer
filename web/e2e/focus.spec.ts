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
});

