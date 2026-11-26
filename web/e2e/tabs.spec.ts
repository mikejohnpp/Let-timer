import { test, expect } from '@playwright/test';
import { setupPage } from './helpers';

test.describe('Tabs transitions with ViewHost (A24)', () => {
  test('single-engine view transition and stable wallpaper', async ({ page }) => {
    await setupPage(page, '/#/today');
    await expect(page.locator('[data-testid="view-tab-today"]')).toBeVisible();

    // Wallpaper layers element should be present
    const wallpaper = page.locator('.wallpaper-layers');
    await expect(wallpaper).toBeAttached();

    // Click Settings in header
    const settingsNav = page.locator('header nav button').filter({ hasText: 'Settings' });
    await settingsNav.click();

    // Old view should be removed and new view visible
    await expect(page.locator('[data-testid="view-tab-today"]')).toHaveCount(0);
    await expect(page.locator('[data-testid="view-tab-settings"]')).toBeVisible();
    await expect(page.locator('[data-testid="view-tab-settings"]')).toHaveCSS('opacity', '1');

    // Wallpaper element was not destroyed or recreated
    await expect(wallpaper).toBeAttached();

    // Return to Today
    const todayNav = page.locator('header nav button').filter({ hasText: 'Today' });
    await todayNav.click();

    await expect(page.locator('[data-testid="view-tab-settings"]')).toHaveCount(0);
    await expect(page.locator('[data-testid="view-tab-today"]')).toBeVisible();
    await expect(page.locator('[data-testid="view-tab-today"]')).toHaveCSS('opacity', '1');
  });

  test.describe('Reduced motion', () => {
    test.use({ reducedMotion: 'reduce' });

    test('transitions cleanly without travel under reduced motion', async ({ page }) => {
      await setupPage(page, '/#/today');
      await expect(page.locator('[data-testid="view-tab-today"]')).toBeVisible();

      const settingsNav = page.locator('header nav button').filter({ hasText: 'Settings' });
      await settingsNav.click();

      await expect(page.locator('[data-testid="view-tab-settings"]')).toBeVisible();
      await expect(page.locator('[data-testid="view-tab-today"]')).toHaveCount(0);
    });
  });
});

