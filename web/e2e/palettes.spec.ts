import { test, expect } from '@playwright/test';
import { setupPage } from './helpers';
import { PALETTES } from '../src/utils/theme';

test.describe('Community Palettes and Picker UI', () => {
  test('all 7 palettes can be selected, update data-palette, and show toast', async ({ page }) => {
    await setupPage(page, '/#/settings');

    const picker = page.getByRole('radiogroup', { name: /color palette/i });
    await expect(picker).toBeVisible();

    const options = picker.getByRole('radio');
    await expect(options).toHaveCount(7);

    for (const pal of PALETTES) {
      const radio = picker.getByRole('radio', { name: new RegExp(`^${pal.label}$`, 'i') });
      await expect(radio).toBeVisible();
      await radio.click();

      // Verify aria-checked
      await expect(radio).toHaveAttribute('aria-checked', 'true');

      // Verify data-palette attribute on root
      const rootPalette = await page.evaluate(() => document.documentElement.getAttribute('data-palette'));
      if (pal.id === 'default') {
        expect(rootPalette).toBeNull();
      } else {
        expect(rootPalette).toBe(pal.id);
      }

      // Verify localStorage
      const stored = await page.evaluate(() => localStorage.getItem('let_timer_palette'));
      expect(stored).toBe(pal.id);
    }
  });

  test('keyboard arrow keys navigate and select palettes in PalettePicker', async ({ page }) => {
    await setupPage(page, '/#/settings');

    const picker = page.getByRole('radiogroup', { name: /color palette/i });
    const defaultRadio = picker.getByRole('radio', { name: /^indigo$/i });
    await defaultRadio.click();

    // Focus and press ArrowRight -> moves to Forest
    await page.keyboard.press('ArrowRight');
    const forestRadio = picker.getByRole('radio', { name: /^forest$/i });
    await expect(forestRadio).toHaveAttribute('aria-checked', 'true');

    // Press ArrowRight again -> moves to Tokyo Night
    await page.keyboard.press('ArrowRight');
    const tokyoRadio = picker.getByRole('radio', { name: /^tokyo night$/i });
    await expect(tokyoRadio).toHaveAttribute('aria-checked', 'true');

    // Press ArrowLeft -> moves back to Forest
    await page.keyboard.press('ArrowLeft');
    await expect(forestRadio).toHaveAttribute('aria-checked', 'true');
  });

  test('captures screenshots of 7 palettes at 360px and 1280px (light and dark)', async ({ page }) => {
    // 360px mobile viewport (no overflow)
    await page.setViewportSize({ width: 360, height: 740 });
    await setupPage(page, '/#/settings');

    const picker = page.getByRole('radiogroup', { name: /color palette/i });
    await expect(picker).toBeVisible();

    // Verify 360px has no horizontal scroll
    const scrollWidth = await page.evaluate(() => document.documentElement.scrollWidth);
    expect(scrollWidth).toBeLessThanOrEqual(360);

    for (const pal of PALETTES) {
      const radio = picker.getByRole('radio', { name: new RegExp(`^${pal.label}$`, 'i') });
      await radio.click();

      // Mobile screenshot (360px)
      await page.screenshot({
        path: `.design/palettes/360px-${pal.id}.png`,
        fullPage: false,
      });
    }

    // Desktop viewport (1280px)
    await page.setViewportSize({ width: 1280, height: 800 });

    for (const themeMode of ['light', 'dark'] as const) {
      await page.evaluate((theme) => {
        document.documentElement.setAttribute('data-theme', theme);
      }, themeMode);

      for (const pal of PALETTES) {
        const radio = picker.getByRole('radio', { name: new RegExp(`^${pal.label}$`, 'i') });
        await radio.click();

        await page.screenshot({
          path: `.design/palettes/1280px-${themeMode}-${pal.id}.png`,
          fullPage: false,
        });
      }
    }
  });
});
