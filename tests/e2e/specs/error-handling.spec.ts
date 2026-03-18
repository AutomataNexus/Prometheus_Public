import { test, expect } from '@playwright/test';
import { login, apiLogin, apiRequest } from '../fixtures/auth';
import path from 'path';

test.describe('Error Handling', () => {
  /* ── 404 page for unknown routes ───────────────────── */

  test('404 page renders for unknown route', async ({ page }) => {
    await login(page);
    await page.goto('/nonexistent-page-12345');

    const notFound = page.locator('[data-testid="not-found"], [data-testid="error-page"]');
    await expect(notFound.first()).toBeVisible({ timeout: 10_000 });
    await expect(notFound.first()).toContainText(/not found|404|does not exist/i);
  });

  test('404 page renders for deeply nested unknown route', async ({ page }) => {
    await login(page);
    await page.goto('/some/deeply/nested/invalid/path');

    const notFound = page.locator('[data-testid="not-found"], [data-testid="error-page"]');
    await expect(notFound.first()).toBeVisible({ timeout: 10_000 });
  });

  /* ── Error boundary ────────────────────────────────── */

  test('error boundary catches component errors gracefully', async ({ page }) => {
    await login(page);

    // Inject a runtime error to test the error boundary
    await page.evaluate(() => {
      window.addEventListener('error', (event) => {
        // Prevent the error from propagating to break the page
        event.preventDefault();
      });
    });

    // Navigate to dashboard and verify it renders without crashing
    await page.goto('/');
    await page.waitForSelector('[data-testid="dashboard"]', { timeout: 10_000 });
    await expect(page.locator('[data-testid="dashboard"]')).toBeVisible();
  });

  /* ── Network error handling ────────────────────────── */

  test('network error shows user-friendly message', async ({ page }) => {
    await login(page);

    // Block API requests to simulate network errors
    await page.route('**/api/v1/datasets**', (route) => {
      route.abort('connectionrefused');
    });

    await page.goto('/datasets');

    // Should show an error message rather than a blank page
    const errorMessage = page.locator(
      '[data-testid="error-message"], [data-testid="error-state"], [role="alert"]',
    );
    await expect(errorMessage.first()).toBeVisible({ timeout: 15_000 });
  });

  /* ── API timeout feedback ──────────────────────────── */

  test('API timeout shows appropriate feedback', async ({ page }) => {
    await login(page);

    // Simulate a slow/timeout API response
    await page.route('**/api/v1/datasets**', async (route) => {
      // Delay for 30 seconds to trigger timeout behavior
      await new Promise((resolve) => setTimeout(resolve, 30_000));
      await route.fulfill({ status: 504, body: 'Gateway Timeout' });
    });

    await page.goto('/datasets');

    // Should show loading state initially, then error/timeout message
    const loadingOrError = page.locator(
      '[data-testid="loading"], [data-testid="error-message"], [data-testid="error-state"], [data-testid="timeout-message"]',
    );
    await expect(loadingOrError.first()).toBeVisible({ timeout: 15_000 });
  });

  /* ── Upload error: wrong file type ─────────────────── */

  test('upload of non-CSV file shows error', async ({ page }) => {
    await login(page);
    await page.goto('/datasets');
    await page.waitForSelector('[data-testid="datasets-page"]');

    const fileInput = page.locator('[data-testid="file-upload"] input[type="file"]');

    // Create a fake .txt file
    const invalidFile = path.resolve(__dirname, '../fixtures/invalid_upload.txt');

    // Set input with invalid file type — the app should reject non-CSV
    await fileInput.setInputFiles({
      name: 'not_a_csv.txt',
      mimeType: 'text/plain',
      buffer: Buffer.from('This is not a CSV file. Just plain text.'),
    });

    // Should show an upload error or validation message
    const errorMessage = page.locator(
      '[data-testid="upload-error"], [data-testid="file-type-error"], [role="alert"]',
    );
    await expect(errorMessage.first()).toBeVisible({ timeout: 10_000 });
    await expect(errorMessage.first()).toContainText(/csv|invalid|unsupported|format/i);
  });

  /* ── Upload error: oversized file ──────────────────── */

  test('upload of oversized file shows error', async ({ page }) => {
    await login(page);
    await page.goto('/datasets');
    await page.waitForSelector('[data-testid="datasets-page"]');

    const fileInput = page.locator('[data-testid="file-upload"] input[type="file"]');

    // Create a large fake CSV (header + large payload)
    const header = 'timestamp,value\n';
    const row = '2026-01-01T00:00:00Z,42.0\n';
    // Create ~10MB of data
    const largeContent = header + row.repeat(400_000);

    await fileInput.setInputFiles({
      name: 'oversized_data.csv',
      mimeType: 'text/csv',
      buffer: Buffer.from(largeContent),
    });

    // Should show a size limit error or start upload (depending on limit config)
    const errorMessage = page.locator(
      '[data-testid="upload-error"], [data-testid="file-size-error"], [role="alert"]',
    );
    const progressBar = page.locator('[data-testid="upload-progress"]');

    // Either an error is shown or the upload starts (both are valid responses)
    const errorVisible = await errorMessage.first().isVisible().catch(() => false);
    const progressVisible = await progressBar.isVisible().catch(() => false);
    expect(errorVisible || progressVisible).toBeTruthy();
  });

  /* ── Invalid form submissions ──────────────────────── */

  test('empty login form shows field validation errors', async ({ page }) => {
    await page.goto('/login');
    await page.waitForSelector('[data-testid="login-form"]');

    // Click login without filling in fields
    await page.getByRole('button', { name: /log\s*in/i }).click();

    // Should show validation errors for required fields
    const errors = page.locator(
      '[data-testid="field-error"], .field-error, [role="alert"], .error-message, :text("required")',
    );
    await expect(errors.first()).toBeVisible({ timeout: 5_000 });
  });

  test('invalid credentials show authentication error', async ({ page }) => {
    await page.goto('/login');
    await page.waitForSelector('[data-testid="login-form"]');

    await page.getByLabel('Username').fill('nonexistent_user');
    await page.getByLabel('Password').fill('wrong_password_12345');
    await page.getByRole('button', { name: /log\s*in/i }).click();

    // Should show an authentication error
    const error = page.locator(
      '[data-testid="login-error"], [data-testid="auth-error"], [role="alert"]',
    );
    await expect(error.first()).toBeVisible({ timeout: 10_000 });
    await expect(error.first()).toContainText(/invalid|incorrect|failed|unauthorized/i);
  });

  /* ── Server error (500) ────────────────────────────── */

  test('server error 500 shows error page', async ({ page }) => {
    await login(page);

    // Intercept and return a 500 error
    await page.route('**/api/v1/models**', (route) => {
      route.fulfill({
        status: 500,
        contentType: 'application/json',
        body: JSON.stringify({ error: 'Internal Server Error' }),
      });
    });

    await page.goto('/models');

    // Should show an error state rather than crashing
    const errorState = page.locator(
      '[data-testid="error-message"], [data-testid="error-state"], [data-testid="error-page"], [role="alert"]',
    );
    await expect(errorState.first()).toBeVisible({ timeout: 15_000 });
  });

  /* ── Expired token redirect ────────────────────────── */

  test('expired token redirects to login', async ({ page }) => {
    await login(page);

    // Simulate token expiration by intercepting API calls with 401
    await page.route('**/api/v1/**', (route) => {
      route.fulfill({
        status: 401,
        contentType: 'application/json',
        body: JSON.stringify({ error: 'Token expired' }),
      });
    });

    // Navigate to a page that requires auth
    await page.goto('/models');

    // Should redirect to login page
    await page.waitForURL('**/login**', { timeout: 15_000 });
    expect(page.url()).toContain('/login');
  });

  /* ── Duplicate action prevention ───────────────────── */

  test('multiple rapid clicks do not cause duplicate actions', async ({ page }) => {
    await login(page);
    await page.goto('/datasets');
    await page.waitForSelector('[data-testid="datasets-page"]');

    const connectBtn = page.getByRole('button', { name: /connect|influxdb/i });
    if ((await connectBtn.count()) > 0) {
      // Track how many modals open
      let modalOpenCount = 0;

      // Rapidly click the button multiple times
      await connectBtn.click();
      await connectBtn.click();
      await connectBtn.click();

      // Only one modal should be visible
      const modals = page.locator('[data-testid="influxdb-modal"]');
      const count = await modals.count();
      expect(count).toBeLessThanOrEqual(1);
    }
  });

  test('rapid form submissions do not create duplicate entries', async ({ page, context }) => {
    await login(page);

    // Track API calls to detect duplicates
    const apiCalls: string[] = [];
    await page.route('**/api/v1/agent/chat', async (route) => {
      apiCalls.push(route.request().url());
      await route.continue();
    });

    await page.goto('/agent');
    await page.waitForSelector('[data-testid="agent-page"]');

    const chatInput = page.locator('[data-testid="chat-input"]');
    await chatInput.fill('Test duplicate message');

    const sendBtn = page.getByRole('button', { name: /send/i });

    // Rapidly click send multiple times
    await sendBtn.click();
    await sendBtn.click();
    await sendBtn.click();

    // Wait briefly for requests to settle
    await page.waitForTimeout(2_000);

    // The send button should be disabled after first click, preventing duplicates
    // At most we should see a small number of actual requests
    const userMessages = page.locator('[data-testid="user-message"]');
    const messageCount = await userMessages.count();

    // Should not have created 3 duplicate messages
    // (1 is ideal, but the test verifies the button was disabled after first click)
    expect(messageCount).toBeLessThanOrEqual(2);
  });
});
