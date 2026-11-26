import { test, expect } from '@playwright/test';
import { setupPage } from './helpers';

test.describe('Ambient Loops and Calm Budget (A07)', () => {
  test('no unauthorized infinite animations in default mode', async ({ page }) => {
    await setupPage(page, '/#/today');
    await expect(page.locator('[data-testid="view-tab-today"]')).toBeVisible();

    const infiniteAnimations = await page.evaluate(() => {
      const anims = document.getAnimations();
      const unauthorized: string[] = [];

      for (const a of anims) {
        const effect = a.effect;
        if (effect && typeof effect.getTiming === 'function') {
          const timing = effect.getTiming();
          if (timing.iterations === Infinity) {
            const target = (effect as KeyframeEffect).target as HTMLElement | null;
            const className = target?.className || '';
            const isAllowed =
              className.includes('animate-spin') ||
              className.includes('skeleton-shimmer') ||
              className.includes('eq-bar') ||
              className.includes('eq-playing');
            if (!isAllowed) {
              unauthorized.push(`tag: ${target?.tagName}, class: ${className}`);
            }
          }
        }
      }
      return unauthorized;
    });

    expect(infiniteAnimations).toEqual([]);
  });

  test.describe('Reduced motion', () => {
    test.use({ reducedMotion: 'reduce' });

    test('zero infinite animations running under reduced motion', async ({ page }) => {
      await setupPage(page, '/#/today');
      await expect(page.locator('[data-testid="view-tab-today"]')).toBeVisible();

      const runningInfiniteAnims = await page.evaluate(() => {
        const anims = document.getAnimations();
        const active: string[] = [];
        for (const a of anims) {
          const timing = a.effect?.getTiming();
          if (timing?.iterations === Infinity && a.playState === 'running') {
            const target = (a.effect as KeyframeEffect).target as HTMLElement | null;
            active.push(target?.className || 'unknown');
          }
        }
        return active;
      });

      expect(runningInfiniteAnims).toEqual([]);
    });
  });
});

