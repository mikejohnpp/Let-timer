import { test, expect } from '@playwright/test';
import { setupPage } from './helpers';

test.describe('Overlays focus return and Radix data-state keyframes', () => {
  test('drawer opens and restores focus to trigger on close', async ({ page }) => {
    await setupPage(page, '/#/today');
    await expect(page.locator('[data-testid="view-tab-today"]')).toBeVisible();

    const addBtn = page.locator('button').filter({ hasText: 'New task' }).first();
    await expect(addBtn).toBeVisible();
    await addBtn.click();

    // Drawer opens
    const drawer = page.locator('[role="dialog"]');
    await expect(drawer).toBeVisible();

    // Close the drawer by clicking cancel or escape
    await page.keyboard.press('Escape');
    await expect(drawer).toHaveCount(0);

    // Focus returns to the trigger
    await expect(addBtn).toBeFocused();
  });

  test.describe('Reduced motion', () => {
    test.use({ reducedMotion: 'reduce' });

    test('no transform animations on overlay under reduced motion', async ({ page }) => {
      await setupPage(page, '/#/today');
      await expect(page.locator('[data-testid="view-tab-today"]')).toBeVisible();

      const addBtn = page.locator('button').filter({ hasText: 'New task' }).first();
      await addBtn.click();

      const drawer = page.locator('[role="dialog"]');
      await expect(drawer).toBeVisible();

      const transform = await drawer.evaluate((el) => window.getComputedStyle(el).transform);
      expect(transform === 'none' || transform.includes('matrix(1, 0, 0, 1, 0, 0)')).toBe(true);
    });
  });
});

