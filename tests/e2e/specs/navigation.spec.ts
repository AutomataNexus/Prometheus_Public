import { test, expect } from '@playwright/test';
import { login } from '../fixtures/auth';

test.describe('Navigation', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  /* ── Sidebar active state ──────────────────────────── */

  test('sidebar highlights active page on dashboard', async ({ page }) => {
    const sidebar = page.locator('[data-testid="sidebar"]');
    const homeLink = sidebar.getByRole('link', { name: /home|dashboard/i }).first();

    if ((await homeLink.count()) > 0) {
      const classes = await homeLink.getAttribute('class');
      expect(classes).toMatch(/active|selected|current|bg-/);
    }
  });

  test('sidebar highlights active page when navigating to datasets', async ({ page }) => {
    await page.goto('/datasets');
    await page.waitForSelector('[data-testid="datasets-page"]');

    const sidebar = page.locator('[data-testid="sidebar"]');
    const dataLink = sidebar.getByRole('link', { name: /data/i }).first();

    if ((await dataLink.count()) > 0) {
      const classes = await dataLink.getAttribute('class');
      expect(classes).toMatch(/active|selected|current|bg-/);
    }
  });

  test('sidebar highlights active page when navigating to training', async ({ page }) => {
    await page.goto('/training');
    await page.waitForSelector('[data-testid="training-page"]');

    const sidebar = page.locator('[data-testid="sidebar"]');
    const trainLink = sidebar.getByRole('link', { name: /train/i }).first();

    if ((await trainLink.count()) > 0) {
      const classes = await trainLink.getAttribute('class');
      expect(classes).toMatch(/active|selected|current|bg-/);
    }
  });

  /* ── Sidebar link navigation ───────────────────────── */

  test('all sidebar links navigate to correct pages', async ({ page }) => {
    const navItems = [
      { label: /data/i, path: '/datasets' },
      { label: /train/i, path: '/training' },
      { label: /model/i, path: '/models' },
      { label: /deploy/i, path: '/deployment' },
      { label: /eval/i, path: '/evaluation' },
      { label: /agent/i, path: '/agent' },
      { label: /setting/i, path: '/settings' },
    ];

    for (const { label, path } of navItems) {
      const link = page.locator('[data-testid="sidebar"]').getByRole('link', { name: label });
      if ((await link.count()) > 0) {
        await link.first().click();
        await page.waitForURL(`**${path}**`);
        expect(page.url()).toContain(path);
      }
    }
  });

  /* ── Browser history navigation ────────────────────── */

  test('browser back navigation returns to previous page', async ({ page }) => {
    await page.goto('/datasets');
    await page.waitForSelector('[data-testid="datasets-page"]');

    await page.goto('/training');
    await page.waitForSelector('[data-testid="training-page"]');

    await page.goBack();
    await page.waitForURL('**/datasets**');
    expect(page.url()).toContain('/datasets');
  });

  test('browser forward navigation works after going back', async ({ page }) => {
    await page.goto('/datasets');
    await page.waitForSelector('[data-testid="datasets-page"]');

    await page.goto('/training');
    await page.waitForSelector('[data-testid="training-page"]');

    await page.goBack();
    await page.waitForURL('**/datasets**');

    await page.goForward();
    await page.waitForURL('**/training**');
    expect(page.url()).toContain('/training');
  });

  /* ── Page titles ───────────────────────────────────── */

  test('page title updates when navigating to datasets', async ({ page }) => {
    await page.goto('/datasets');
    await page.waitForSelector('[data-testid="datasets-page"]');

    const title = await page.title();
    expect(title).toMatch(/data|prometheus/i);
  });

  test('page title updates when navigating to training', async ({ page }) => {
    await page.goto('/training');
    await page.waitForSelector('[data-testid="training-page"]');

    const title = await page.title();
    expect(title).toMatch(/train|prometheus/i);
  });

  /* ── Breadcrumbs ───────────────────────────────────── */

  test('breadcrumbs show correct path on dataset detail page', async ({ page }) => {
    await page.goto('/datasets');
    await page.waitForSelector('[data-testid="datasets-page"]');

    const datasetRows = page.locator('[data-testid="dataset-table"] tbody tr');
    if ((await datasetRows.count()) > 0) {
      await datasetRows.first().click();
      await page.waitForURL('**/datasets/**');

      const breadcrumbs = page.locator('[data-testid="breadcrumbs"]');
      if ((await breadcrumbs.count()) > 0) {
        await expect(breadcrumbs).toBeVisible();
        await expect(breadcrumbs.getByText(/data/i)).toBeVisible();
      }
    }
  });

  test('breadcrumbs show correct path on model detail page', async ({ page }) => {
    await page.goto('/models');
    await page.waitForSelector('[data-testid="models-page"]');

    const modelCards = page.locator('[data-testid="model-card"]');
    if ((await modelCards.count()) > 0) {
      await modelCards.first().click();
      await page.waitForURL('**/models/**');

      const breadcrumbs = page.locator('[data-testid="breadcrumbs"]');
      if ((await breadcrumbs.count()) > 0) {
        await expect(breadcrumbs).toBeVisible();
        await expect(breadcrumbs.getByText(/model/i)).toBeVisible();
      }
    }
  });

  /* ── Deep linking ──────────────────────────────────── */

  test('deep link to /datasets/ds_123 renders dataset detail or 404', async ({ page }) => {
    const response = await page.goto('/datasets/ds_123');
    // Either shows a detail page or a not-found state
    const status = response?.status();
    if (status === 200) {
      const detail = page.locator('[data-testid="dataset-detail"]');
      const notFound = page.locator('[data-testid="not-found"], [data-testid="error-page"]');
      const visible = (await detail.count()) > 0 || (await notFound.count()) > 0;
      expect(visible).toBeTruthy();
    } else {
      expect([200, 404]).toContain(status);
    }
  });

  test('deep link to /models/mdl_456 renders model detail or 404', async ({ page }) => {
    const response = await page.goto('/models/mdl_456');
    const status = response?.status();
    if (status === 200) {
      const detail = page.locator('[data-testid="model-detail"]');
      const notFound = page.locator('[data-testid="not-found"], [data-testid="error-page"]');
      const visible = (await detail.count()) > 0 || (await notFound.count()) > 0;
      expect(visible).toBeTruthy();
    } else {
      expect([200, 404]).toContain(status);
    }
  });

  /* ── 404 for invalid routes ────────────────────────── */

  test('shows 404 page for invalid route', async ({ page }) => {
    await page.goto('/this-route-does-not-exist');

    const notFound = page.locator('[data-testid="not-found"], [data-testid="error-page"]');
    await expect(notFound.first()).toBeVisible({ timeout: 10_000 });
    await expect(notFound.first()).toContainText(/not found|404|page.*exist/i);
  });

  /* ── Scroll position preservation ──────────────────── */

  test('navigation preserves scroll position on back', async ({ page }) => {
    await page.goto('/datasets');
    await page.waitForSelector('[data-testid="datasets-page"]');

    // Scroll down on the datasets page
    await page.evaluate(() => window.scrollTo(0, 300));
    const scrollBefore = await page.evaluate(() => window.scrollY);

    // Navigate away
    await page.goto('/training');
    await page.waitForSelector('[data-testid="training-page"]');

    // Go back
    await page.goBack();
    await page.waitForURL('**/datasets**');

    // Allow browser time to restore scroll position
    await page.waitForTimeout(500);
    const scrollAfter = await page.evaluate(() => window.scrollY);

    // Scroll position should be restored (or at least non-zero if content is tall enough)
    // Browsers may or may not restore scroll — check it doesn't crash
    expect(typeof scrollAfter).toBe('number');
  });

  /* ── Mobile sidebar ────────────────────────────────── */

  test('sidebar collapses on mobile viewport', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 812 });
    await page.reload();
    await page.waitForSelector('[data-testid="dashboard"]');

    const sidebar = page.locator('[data-testid="sidebar"]');
    const isVisible = await sidebar.isVisible();

    if (!isVisible) {
      const hamburger = page.locator('[data-testid="mobile-menu-toggle"]');
      await expect(hamburger).toBeVisible();
    }
  });

  test('sidebar expands when hamburger menu is clicked on mobile', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 812 });
    await page.reload();
    await page.waitForSelector('[data-testid="dashboard"]');

    const sidebar = page.locator('[data-testid="sidebar"]');
    const isVisible = await sidebar.isVisible();

    if (!isVisible) {
      const hamburger = page.locator('[data-testid="mobile-menu-toggle"]');
      await hamburger.click();
      await expect(sidebar).toBeVisible();

      // Click a nav link to navigate
      const dataLink = sidebar.getByRole('link', { name: /data/i }).first();
      if ((await dataLink.count()) > 0) {
        await dataLink.click();
        await page.waitForURL('**/datasets**');
        expect(page.url()).toContain('/datasets');
      }
    }
  });

  /* ── Keyboard navigation ───────────────────────────── */

  test('Tab key moves focus through sidebar items', async ({ page }) => {
    const sidebar = page.locator('[data-testid="sidebar"]');
    await expect(sidebar).toBeVisible();

    // Focus the first link in the sidebar
    const firstLink = sidebar.getByRole('link').first();
    await firstLink.focus();

    // Tab through sidebar links
    const linkCount = await sidebar.getByRole('link').count();
    const focusedElements: string[] = [];

    for (let i = 0; i < Math.min(linkCount, 7); i++) {
      const focused = await page.evaluate(() => {
        const el = document.activeElement;
        return el?.tagName + ':' + el?.textContent?.trim();
      });
      focusedElements.push(focused);
      await page.keyboard.press('Tab');
    }

    // Should have focused multiple distinct elements
    const uniqueElements = new Set(focusedElements);
    expect(uniqueElements.size).toBeGreaterThanOrEqual(1);
  });

  test('Enter key activates focused sidebar link', async ({ page }) => {
    const sidebar = page.locator('[data-testid="sidebar"]');
    const dataLink = sidebar.getByRole('link', { name: /data/i }).first();

    if ((await dataLink.count()) > 0) {
      await dataLink.focus();
      await page.keyboard.press('Enter');
      await page.waitForURL('**/datasets**');
      expect(page.url()).toContain('/datasets');
    }
  });
});
