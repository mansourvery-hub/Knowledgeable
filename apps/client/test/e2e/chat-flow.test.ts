import { test, expect } from '@playwright/test';
import { ServerManager } from './helpers/server-manager';

const server = new ServerManager();

test.beforeAll(async () => {
  await server.startBackend();
});

test.afterAll(async () => {
  await server.stopBackend();
});

test('Chat flow: sends message and receives response', async ({ page }) => {
  // Navigate to Flutter app (assuming it's running via `fvm flutter run` or static build)
  // For E2E testing, we usually expect the app to be served by a web server
  await page.goto('/');

  // Simulate user interaction with the flyer chat widget
  // The input field in flyer chat usually has a specific structure.
  // We can try to find it by common attributes or placeholders.
  const chatInput = page.getByRole('textbox'); 
  await chatInput.fill('What is a weak dependency?');
  
  // Press send button
  await page.getByRole('button', { name: 'Send' }).click();

  // Verify response
  // We look for a message that is not from the user
  const messageResponse = page.locator('.message-bubble').last();
  await expect(messageResponse).toBeVisible({ timeout: 20000 });
});
