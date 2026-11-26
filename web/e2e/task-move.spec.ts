import { test, expect } from '@playwright/test';
import { setupPage } from './helpers';

test.describe('Task Row Move and FLIP (R2 / A17)', () => {
  test('task complete travels without duplicating nodes, exactly one node at rest', async ({ page }) => {
    await setupPage(page, '/#/today');
    await expect(page.locator('[data-testid="view-tab-today"]')).toBeVisible();

    // Find the pending task row
    const pendingRow = page.locator('[data-task-id]').filter({ hasText: 'Review pull request' }).first();
    await expect(pendingRow).toBeVisible();

    const taskId = await pendingRow.getAttribute('data-task-id');
    expect(taskId).toBeTruthy();

    // Click the complete button
    const completeBtn = pendingRow.locator('button[aria-label="Mark task done"]');
    await completeBtn.click();

    // Sample node count over 600ms every 50ms
    const samples: number[] = [];
    for (let i = 0; i < 12; i++) {
      const count = await page.locator(`[data-task-id="${taskId}"]`).count();
      samples.push(count);
      // At no point should there be more than two nodes with this id
      expect(count).toBeLessThanOrEqual(2);
      await page.waitForTimeout(50);
    }

    // After 600ms transition completes, exactly one node exists
    await expect(page.locator(`[data-task-id="${taskId}"]`)).toHaveCount(1);

    // And it has status completed
    const finalRow = page.locator(`[data-task-id="${taskId}"]`);
    await expect(finalRow.locator('button[aria-label="Reopen task"]')).toBeVisible();
  });

  test.describe('Reduced motion', () => {
    test.use({ reducedMotion: 'reduce' });

    test('row moves with instant layout and no transform', async ({ page }) => {
      await setupPage(page, '/#/today');
      await expect(page.locator('[data-testid="view-tab-today"]')).toBeVisible();

      const taskRow = page.locator('[data-task-id]').filter({ hasText: 'Brainstorm ideas' }).first();
      await expect(taskRow).toBeVisible();

      const taskId = await taskRow.getAttribute('data-task-id');
      const completeBtn = taskRow.locator('button[aria-label="Mark task done"]');
      await completeBtn.click();

      // Wait for task completion to finish
      const row = page.locator(`[data-task-id="${taskId}"]`);
      await expect(row.locator('button[aria-label="Reopen task"]')).toBeVisible();
      await expect(row).toHaveCount(1);

      const transform = await row.evaluate((el) => window.getComputedStyle(el).transform);
      expect(transform === 'none' || transform.includes('matrix(1, 0, 0, 1, 0, 0)')).toBe(true);
    });
  });
});

