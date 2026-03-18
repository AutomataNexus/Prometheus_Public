import { test as base, expect, type Page, type BrowserContext } from '@playwright/test';

/* ──────────────────────────────────────────────────────────────
 * Default test credentials
 * ────────────────────────────────────────────────────────────── */
export const TEST_ADMIN = {
  username: process.env.TEST_ADMIN_USER ?? 'admin',
  password: process.env.TEST_ADMIN_PASS ?? 'admin_password',
};

export const TEST_OPERATOR = {
  username: process.env.TEST_OPERATOR_USER ?? 'operator',
  password: process.env.TEST_OPERATOR_PASS ?? 'operator_password',
};

export const TEST_VIEWER = {
  username: process.env.TEST_VIEWER_USER ?? 'viewer',
  password: process.env.TEST_VIEWER_PASS ?? 'viewer_password',
};

/* ──────────────────────────────────────────────────────────────
 * Login helper — fills the login form and waits for navigation
 * to the dashboard.
 * ────────────────────────────────────────────────────────────── */
export async function login(
  page: Page,
  credentials: { username: string; password: string } = TEST_ADMIN,
): Promise<string> {
  await page.goto('/login');
  await page.waitForSelector('[data-testid="login-form"]');

  await page.getByLabel('Username').fill(credentials.username);
  await page.getByLabel('Password').fill(credentials.password);
  await page.getByRole('button', { name: /log\s*in/i }).click();

  // Wait for redirect to dashboard
  await page.waitForURL('**/');
  await expect(page.locator('[data-testid="dashboard"]')).toBeVisible({ timeout: 10_000 });

  // Extract the bearer token from localStorage for API-level assertions
  const token = await page.evaluate(() => localStorage.getItem('prometheus_token') ?? '');
  return token;
}

/* ──────────────────────────────────────────────────────────────
 * Logout helper — clicks the user menu and logs out
 * ────────────────────────────────────────────────────────────── */
export async function logout(page: Page): Promise<void> {
  await page.getByTestId('user-menu-button').click();
  await page.getByRole('menuitem', { name: /log\s*out/i }).click();
  await page.waitForURL('**/login');
}

/* ──────────────────────────────────────────────────────────────
 * Auth fixture — extends the base test so every spec in the
 * `authenticatedTest` suite starts already logged-in.
 * ────────────────────────────────────────────────────────────── */
type AuthFixtures = {
  authenticatedPage: Page;
  authToken: string;
};

export const authenticatedTest = base.extend<AuthFixtures>({
  authenticatedPage: async ({ page }, use) => {
    await login(page);
    await use(page);
  },
  authToken: async ({ page }, use) => {
    const token = await login(page);
    await use(token);
  },
});

/* ──────────────────────────────────────────────────────────────
 * API helper — makes authenticated API requests directly
 * (bypassing the UI) for setup/teardown operations.
 * ────────────────────────────────────────────────────────────── */
export async function apiLogin(
  context: BrowserContext,
  credentials: { username: string; password: string } = TEST_ADMIN,
): Promise<string> {
  const response = await context.request.post('/api/v1/auth/login', {
    data: {
      username: credentials.username,
      password: credentials.password,
    },
  });
  expect(response.ok()).toBeTruthy();
  const body = await response.json();
  return body.token;
}

export async function apiRequest(
  context: BrowserContext,
  method: string,
  path: string,
  token: string,
  data?: unknown,
) {
  const options: Record<string, unknown> = {
    headers: { Authorization: `Bearer ${token}` },
  };
  if (data !== undefined) {
    options.data = data;
  }

  switch (method.toUpperCase()) {
    case 'GET':
      return context.request.get(path, options);
    case 'POST':
      return context.request.post(path, options);
    case 'DELETE':
      return context.request.delete(path, options);
    default:
      throw new Error(`Unsupported method: ${method}`);
  }
}
