import { test, expect } from '@playwright/test';
import { login, apiLogin, TEST_ADMIN } from '../fixtures/auth';

test.describe('Settings Page', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
    await page.goto('/settings');
    await page.waitForSelector('[data-testid="settings-page"]');
  });

  /* ── Profile section ────────────────────────────────── */

  test('displays current user profile', async ({ page }) => {
    const profileSection = page.locator('[data-testid="profile-section"]');
    await expect(profileSection).toBeVisible();

    // Username
    const usernameField = profileSection.locator('[data-testid="profile-username"]');
    await expect(usernameField).toBeVisible();
    const usernameText = await usernameField.inputValue?.() ?? await usernameField.textContent();
    expect(usernameText).toContain(TEST_ADMIN.username);

    // Email field
    const emailField = profileSection.locator('[data-testid="profile-email"]');
    await expect(emailField).toBeVisible();

    // Role display
    const roleField = profileSection.locator('[data-testid="profile-role"]');
    await expect(roleField).toBeVisible();
    const roleText = await roleField.textContent();
    expect(roleText).toMatch(/admin|operator|viewer/i);
  });

  test('updates user email', async ({ page }) => {
    const profileSection = page.locator('[data-testid="profile-section"]');
    const emailInput = profileSection.locator('[data-testid="profile-email"] input, input[name="email"]');

    if ((await emailInput.count()) > 0) {
      await emailInput.clear();
      await emailInput.fill('updated@example.com');

      const saveBtn = profileSection.getByRole('button', { name: /save|update/i });
      await saveBtn.click();

      // Success notification
      const toast = page.locator('[data-testid="toast-success"]');
      await expect(toast).toBeVisible({ timeout: 5_000 });
    }
  });

  /* ── API keys section ───────────────────────────────── */

  test('displays API key management section', async ({ page }) => {
    const apiSection = page.locator('[data-testid="api-keys-section"]');
    await expect(apiSection).toBeVisible();
  });

  test('generates new API key', async ({ page }) => {
    const apiSection = page.locator('[data-testid="api-keys-section"]');
    const generateBtn = apiSection.getByRole('button', { name: /generate|create|new/i });

    if ((await generateBtn.count()) > 0) {
      await generateBtn.click();

      // New key should be displayed (usually shown once)
      const newKeyDisplay = page.locator('[data-testid="new-api-key"]');
      await expect(newKeyDisplay).toBeVisible({ timeout: 5_000 });

      const keyText = await newKeyDisplay.textContent();
      expect(keyText?.length).toBeGreaterThan(10);
    }
  });

  test('revokes existing API key', async ({ page }) => {
    const apiSection = page.locator('[data-testid="api-keys-section"]');
    const keyRows = apiSection.locator('[data-testid="api-key-row"]');
    const count = await keyRows.count();

    if (count > 0) {
      const revokeBtn = keyRows.first().getByRole('button', { name: /revoke|delete|remove/i });
      if ((await revokeBtn.count()) > 0) {
        await revokeBtn.click();

        // Confirmation
        const dialog = page.locator('[data-testid="confirm-dialog"]');
        if ((await dialog.count()) > 0) {
          await dialog.getByRole('button', { name: /confirm|revoke|yes/i }).click();
        }

        // Key should be removed or marked as revoked
        const toast = page.locator('[data-testid="toast-success"]');
        await expect(toast).toBeVisible({ timeout: 5_000 });
      }
    }
  });

  test('masks API key values by default', async ({ page }) => {
    const apiSection = page.locator('[data-testid="api-keys-section"]');
    const keyValues = apiSection.locator('[data-testid="api-key-value"]');
    const count = await keyValues.count();

    for (let i = 0; i < count; i++) {
      const text = await keyValues.nth(i).textContent();
      // Should be masked with dots or asterisks
      expect(text).toMatch(/\*{3,}|\.{3,}|[a-z0-9]{4}\*+/i);
    }
  });

  /* ── Gradient AI configuration ──────────────────────── */

  test('configures Gradient AI connection settings', async ({ page }) => {
    const gradientSection = page.locator('[data-testid="gradient-config-section"]');
    await expect(gradientSection).toBeVisible();

    // API token field
    const tokenInput = gradientSection.locator(
      'input[name="gradient_api_token"], [data-testid="gradient-token-input"]',
    );
    await expect(tokenInput).toBeVisible();

    // Agent ID field
    const agentIdInput = gradientSection.locator(
      'input[name="gradient_agent_id"], [data-testid="gradient-agent-id-input"]',
    );
    await expect(agentIdInput).toBeVisible();
  });

  test('saves Gradient configuration', async ({ page }) => {
    const gradientSection = page.locator('[data-testid="gradient-config-section"]');

    const tokenInput = gradientSection.locator(
      'input[name="gradient_api_token"], [data-testid="gradient-token-input"]',
    );
    const agentIdInput = gradientSection.locator(
      'input[name="gradient_agent_id"], [data-testid="gradient-agent-id-input"]',
    );

    if ((await tokenInput.count()) > 0) {
      await tokenInput.clear();
      await tokenInput.fill('dop_v1_test_token_12345');
    }
    if ((await agentIdInput.count()) > 0) {
      await agentIdInput.clear();
      await agentIdInput.fill('agent_test_id_67890');
    }

    const saveBtn = gradientSection.getByRole('button', { name: /save|update/i });
    await saveBtn.click();

    const toast = page.locator('[data-testid="toast-success"]');
    await expect(toast).toBeVisible({ timeout: 5_000 });
  });

  test('tests Gradient connection', async ({ page }) => {
    const gradientSection = page.locator('[data-testid="gradient-config-section"]');
    const testBtn = gradientSection.getByRole('button', { name: /test|verify|check/i });

    if ((await testBtn.count()) > 0) {
      await testBtn.click();

      // Should show connection test result
      const result = gradientSection.locator('[data-testid="connection-test-result"]');
      await expect(result).toBeVisible({ timeout: 15_000 });
    }
  });

  /* ── Password change ────────────────────────────────── */

  test('changes user password', async ({ page }) => {
    const passwordSection = page.locator('[data-testid="password-section"]');
    await expect(passwordSection).toBeVisible();

    const currentPassword = passwordSection.locator(
      'input[name="current_password"], [data-testid="current-password"]',
    );
    const newPassword = passwordSection.locator(
      'input[name="new_password"], [data-testid="new-password"]',
    );
    const confirmPassword = passwordSection.locator(
      'input[name="confirm_password"], [data-testid="confirm-password"]',
    );

    await currentPassword.fill(TEST_ADMIN.password);
    await newPassword.fill('new_secure_password_456!');
    await confirmPassword.fill('new_secure_password_456!');

    const changeBtn = passwordSection.getByRole('button', { name: /change|update|save/i });
    await changeBtn.click();

    const toast = page.locator('[data-testid="toast-success"]');
    await expect(toast).toBeVisible({ timeout: 5_000 });

    // Revert password back for other tests
    await currentPassword.fill('new_secure_password_456!');
    await newPassword.fill(TEST_ADMIN.password);
    await confirmPassword.fill(TEST_ADMIN.password);
    await changeBtn.click();
    await expect(toast).toBeVisible({ timeout: 5_000 });
  });

  test('shows error for mismatched password confirmation', async ({ page }) => {
    const passwordSection = page.locator('[data-testid="password-section"]');

    const currentPassword = passwordSection.locator(
      'input[name="current_password"], [data-testid="current-password"]',
    );
    const newPassword = passwordSection.locator(
      'input[name="new_password"], [data-testid="new-password"]',
    );
    const confirmPassword = passwordSection.locator(
      'input[name="confirm_password"], [data-testid="confirm-password"]',
    );

    await currentPassword.fill(TEST_ADMIN.password);
    await newPassword.fill('new_password_123');
    await confirmPassword.fill('different_password_456');

    const changeBtn = passwordSection.getByRole('button', { name: /change|update|save/i });
    await changeBtn.click();

    // Should show mismatch error
    const error = page.locator('[data-testid="password-error"]');
    await expect(error).toBeVisible({ timeout: 5_000 });
    await expect(error).toContainText(/match|mismatch|do not match/i);
  });

  test('shows error for incorrect current password', async ({ page }) => {
    const passwordSection = page.locator('[data-testid="password-section"]');

    const currentPassword = passwordSection.locator(
      'input[name="current_password"], [data-testid="current-password"]',
    );
    const newPassword = passwordSection.locator(
      'input[name="new_password"], [data-testid="new-password"]',
    );
    const confirmPassword = passwordSection.locator(
      'input[name="confirm_password"], [data-testid="confirm-password"]',
    );

    await currentPassword.fill('wrong_current_password');
    await newPassword.fill('some_new_password');
    await confirmPassword.fill('some_new_password');

    const changeBtn = passwordSection.getByRole('button', { name: /change|update|save/i });
    await changeBtn.click();

    const error = page.locator('[data-testid="password-error"]');
    await expect(error).toBeVisible({ timeout: 5_000 });
    await expect(error).toContainText(/incorrect|wrong|invalid/i);
  });

  /* ── Settings page sections ─────────────────────────── */

  test('all settings sections are visible', async ({ page }) => {
    const sections = [
      '[data-testid="profile-section"]',
      '[data-testid="api-keys-section"]',
      '[data-testid="gradient-config-section"]',
      '[data-testid="password-section"]',
    ];

    for (const selector of sections) {
      await expect(page.locator(selector)).toBeVisible();
    }
  });

  test('settings page is scrollable on mobile', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 812 });
    await page.reload();
    await page.waitForSelector('[data-testid="settings-page"]');

    // All sections should still be accessible via scrolling
    const passwordSection = page.locator('[data-testid="password-section"]');
    await passwordSection.scrollIntoViewIfNeeded();
    await expect(passwordSection).toBeVisible();
  });
});
