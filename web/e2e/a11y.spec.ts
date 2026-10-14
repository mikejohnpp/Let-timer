import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { setupPage } from './helpers';

test.describe('Accessibility Audits (Axe Core)', () => {
  const views = ['/#/today', '/#/workspaces', '/#/media', '/#/settings', '/#/focus'];

  for (const view of views) {
    test(`zero serious or critical violations on ${view}`, async ({ page }) => {
      await setupPage(page, view);
      await page.waitForTimeout(500);

      const results = await new AxeBuilder({ page })
        .withTags(['wcag2a', 'wcag2aa'])
        .analyze();

      const severe = results.violations.filter(
        (v) => v.impact === 'serious' || v.impact === 'critical'
      );

      expect(severe).toEqual([]);
    });
  }

  test('zero serious or critical violations with drawer open', async ({ page }) => {
    await setupPage(page, '/#/today');
    const addBtn = page.locator('button').filter({ hasText: 'New task' }).first();
    await addBtn.click();
    await expect(page.locator('[role="dialog"]')).toBeVisible();

    const results = await new AxeBuilder({ page })
      .withTags(['wcag2a', 'wcag2aa'])
      .analyze();

    const severe = results.violations.filter(
      (v) => v.impact === 'serious' || v.impact === 'critical'
    );

    expect(severe).toEqual([]);
  });
});

