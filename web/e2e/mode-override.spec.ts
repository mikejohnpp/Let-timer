import { test, expect } from '@playwright/test';
import { setupPage } from './helpers';

test.describe('Mode Override: Full motion overrides OS-reduced preference', () => {
  test.use({ reducedMotion: 'reduce' });

  test('with data-motion="full", animations still execute despite OS reduced preference', async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem('let_timer_motion', 'full');
    });
    await setupPage(page, '/#/today');

    // Document element has data-motion="full"
    const dataMotion = await page.evaluate(() => document.documentElement.getAttribute('data-motion'));
    expect(dataMotion).toBe('full');

    const durFast = await page.evaluate(() => {
      return getComputedStyle(document.documentElement).getPropertyValue('--dur-fast').trim();
    });
    expect(durFast).toBe('120ms');
  });

  test('with system mode and OS reduced preference, reduced motion tokens apply', async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.removeItem('let_timer_motion');
    });
    await setupPage(page, '/#/today');

    const dataMotion = await page.evaluate(() => document.documentElement.getAttribute('data-motion'));
    expect(dataMotion).toBeNull();

    const durFast = await page.evaluate(() => {
      return getComputedStyle(document.documentElement).getPropertyValue('--dur-fast').trim();
    });
    expect(durFast).toBe('100ms');
  });
});

