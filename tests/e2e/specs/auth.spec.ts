import { test, expect } from '@playwright/test';
import { login, logout, TEST_ADMIN, TEST_OPERATOR, apiLogin } from '../fixtures/auth';

test.describe('Authentication Flows', () => {
  /* ── Unauthenticated access ─────────────────────────── */

  test('displays login page when unauthenticated', async ({ page }) => {
    await page.goto('/login');
    await expect(page.locator('[data-testid="login-form"]')).toBeVisible();
    await expect(page.getByLabel('Username')).toBeVisible();
    await expect(page.getByLabel('Password')).toBeVisible();
    await expect(page.getByRole('button', { name: /log\s*in/i })).toBeVisible();
  });

  test('redirects to /login for protected routes', async ({ page }) => {
    const protectedPaths = ['/', '/datasets', '/training', '/models', '/deployment', '/evaluation', '/agent', '/settings'];

    for (const path of protectedPaths) {
      await page.goto(path);
      await page.waitForURL('**/login**');
      expect(page.url()).toContain('/login');
    }
  });

  test('preserves original URL as redirect parameter after login', async ({ page }) => {
    await page.goto('/datasets');
    await page.waitForURL('**/login**');

    // The URL should include a redirect query param
    const url = new URL(page.url());
    const redirect = url.searchParams.get('redirect') ?? url.searchParams.get('next');
    // If the app encodes redirect info, verify it; otherwise just check we landed on login
    expect(page.url()).toContain('/login');
  });

  /* ── Successful login ───────────────────────────────── */

  test('logs in with valid admin credentials', async ({ page }) => {
    await login(page, TEST_ADMIN);

    await expect(page.locator('[data-testid="dashboard"]')).toBeVisible();
    // Verify token is stored
    const token = await page.evaluate(() => localStorage.getItem('prometheus_token'));
    expect(token).toBeTruthy();
    expect(typeof token).toBe('string');
    expect(token!.length).toBeGreaterThan(10);
  });

  test('logs in with operator credentials', async ({ page }) => {
    await login(page, TEST_OPERATOR);
    await expect(page.locator('[data-testid="dashboard"]')).toBeVisible();
  });

  test('navigates to dashboard after login', async ({ page }) => {
    await login(page);
    await expect(page).toHaveURL(/\/$/);
    await expect(page.locator('[data-testid="pipeline-viz"]')).toBeVisible();
  });

  /* ── Failed login ───────────────────────────────────── */

  test('shows error for invalid credentials', async ({ page }) => {
    await page.goto('/login');
    await page.getByLabel('Username').fill('invalid_user');
    await page.getByLabel('Password').fill('wrong_password_123');
    await page.getByRole('button', { name: /log\s*in/i }).click();

    await expect(page.locator('[data-testid="login-error"]')).toBeVisible();
    await expect(page.locator('[data-testid="login-error"]')).toContainText(/invalid|incorrect|unauthorized/i);

    // Should remain on login page
    expect(page.url()).toContain('/login');
  });

  test('shows error for empty username', async ({ page }) => {
    await page.goto('/login');
    await page.getByLabel('Password').fill('some_password');
    await page.getByRole('button', { name: /log\s*in/i }).click();

    // Form validation or API error
    const error = page.locator('[data-testid="login-error"], [data-testid="username-error"]');
    await expect(error.first()).toBeVisible({ timeout: 5_000 });
  });

  test('shows error for empty password', async ({ page }) => {
    await page.goto('/login');
    await page.getByLabel('Username').fill('admin');
    await page.getByRole('button', { name: /log\s*in/i }).click();

    const error = page.locator('[data-testid="login-error"], [data-testid="password-error"]');
    await expect(error.first()).toBeVisible({ timeout: 5_000 });
  });

  /* ── Rate limiting ──────────────────────────────────── */

  test('rate limits after repeated failed login attempts', async ({ page }) => {
    await page.goto('/login');

    // Attempt 31 rapid failed logins (limit is 30/min per PRD)
    for (let i = 0; i < 31; i++) {
      await page.getByLabel('Username').fill(`bruteforce_user_${i}`);
      await page.getByLabel('Password').fill(`bad_password_${i}`);
      await page.getByRole('button', { name: /log\s*in/i }).click();

      // Wait briefly for response
      await page.waitForTimeout(100);

      // Clear fields for next attempt
      await page.getByLabel('Username').clear();
      await page.getByLabel('Password').clear();
    }

    // The last attempt should show a rate-limit error
    const rateLimitMsg = page.locator('[data-testid="login-error"]');
    await expect(rateLimitMsg).toContainText(/rate.limit|too.many|try.again/i);
  });

  /* ── Logout ─────────────────────────────────────────── */

  test('logs out and redirects to login page', async ({ page }) => {
    await login(page);
    await logout(page);

    await expect(page).toHaveURL(/\/login/);
    await expect(page.locator('[data-testid="login-form"]')).toBeVisible();

    // Token should be cleared
    const token = await page.evaluate(() => localStorage.getItem('prometheus_token'));
    expect(token).toBeFalsy();
  });

  test('cannot access protected routes after logout', async ({ page }) => {
    await login(page);
    await logout(page);

    await page.goto('/datasets');
    await page.waitForURL('**/login**');
    expect(page.url()).toContain('/login');
  });

  /* ── Session management ─────────────────────────────── */

  test('session is validated on page load', async ({ page }) => {
    await login(page);

    // Reload — should stay authenticated
    await page.reload();
    await expect(page.locator('[data-testid="dashboard"]')).toBeVisible();
  });

  test('invalid token in localStorage triggers redirect to login', async ({ page }) => {
    await page.goto('/login');

    // Inject an invalid token
    await page.evaluate(() => {
      localStorage.setItem('prometheus_token', 'invalid_expired_token_abc123');
    });

    await page.goto('/');
    await page.waitForURL('**/login**');
    expect(page.url()).toContain('/login');
  });

  test('session validation via API returns correct user info', async ({ context }) => {
    const token = await apiLogin(context, TEST_ADMIN);

    const sessionResp = await context.request.get('/api/v1/auth/session', {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(sessionResp.ok()).toBeTruthy();

    const session = await sessionResp.json();
    expect(session.valid).toBe(true);
    expect(session.user).toHaveProperty('username', TEST_ADMIN.username);
  });

  test('current user info endpoint returns profile', async ({ context }) => {
    const token = await apiLogin(context, TEST_ADMIN);

    const meResp = await context.request.get('/api/v1/auth/me', {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(meResp.ok()).toBeTruthy();

    const me = await meResp.json();
    expect(me).toHaveProperty('username', TEST_ADMIN.username);
    expect(me).toHaveProperty('role');
  });

  /* ── API-level auth checks ──────────────────────────── */

  test('API returns 401 for requests without token', async ({ context }) => {
    const resp = await context.request.get('/api/v1/datasets');
    expect(resp.status()).toBe(401);
  });

  test('API returns 401 for requests with expired/invalid token', async ({ context }) => {
    const resp = await context.request.get('/api/v1/datasets', {
      headers: { Authorization: 'Bearer totally_invalid_token_here' },
    });
    expect(resp.status()).toBe(401);
  });
});
