import { test, expect } from '@playwright/test';

// See here how to get started:
// https://playwright.dev/docs/intro
test('visits the app root url', async ({ page }) => {
  await page.goto('/');
  // The title can be either English or Chinese depending on browser locale
  await expect(page.locator('h1')).toHaveText(/XMPKit WebAssembly Demo/);
})
