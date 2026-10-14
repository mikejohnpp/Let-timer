import { Page } from '@playwright/test';

export const E2E_TOKEN = 'e2e-test-token';

export async function setupPage(page: Page, path: string = '/#/today') {
  await page.addInitScript((token) => {
    localStorage.setItem('let_timer_token', token);
  }, E2E_TOKEN);
  await page.goto(path);
}

