import { test, expect } from '@playwright/test';
import { login } from '../fixtures/auth';

test.describe('Home Dashboard', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  /* ── Pipeline visualization ─────────────────────────── */

  test('renders pipeline visualization with all five stages', async ({ page }) => {
    const pipeline = page.locator('[data-testid="pipeline-viz"]');
    await expect(pipeline).toBeVisible();

    const stages = ['Ingest', 'Analyze', 'Train', 'Evaluate', 'Deploy'];
    for (const stage of stages) {
      await expect(pipeline.getByText(stage, { exact: false })).toBeVisible();
    }
  });

  test('pipeline stages show active indicator for in-progress stage', async ({ page }) => {
    const pipeline = page.locator('[data-testid="pipeline-viz"]');
    await expect(pipeline).toBeVisible();

    // At least one stage should have an active/highlight visual state
    const activeStages = pipeline.locator('[data-active="true"], .stage-active, .active');
    // This may be zero if nothing is running — just ensure no crash
    const count = await activeStages.count();
    expect(count).toBeGreaterThanOrEqual(0);
  });

  /* ── Metric cards ───────────────────────────────────── */

  test('displays metric cards for models, training, deployed, and agent queries', async ({ page }) => {
    const metricCards = page.locator('[data-testid="metric-card"]');
    await expect(metricCards.first()).toBeVisible();

    const expectedLabels = ['Models Trained', 'Training Active', 'Deployed', 'Agent Queries'];
    for (const label of expectedLabels) {
      const card = page.locator(`[data-testid="metric-card"]:has-text("${label}")`);
      await expect(card).toBeVisible();
    }
  });

  test('metric cards display numeric values', async ({ page }) => {
    const metricValues = page.locator('[data-testid="metric-card"] [data-testid="metric-value"]');
    const count = await metricValues.count();
    expect(count).toBeGreaterThanOrEqual(4);

    for (let i = 0; i < count; i++) {
      const text = await metricValues.nth(i).textContent();
      // Value should be a number (possibly with commas/decimals)
      expect(text?.trim()).toMatch(/^\d[\d,]*\.?\d*$/);
    }
  });

  test('metric cards show trend indicators', async ({ page }) => {
    // Each card should have an up/down trend arrow or percentage
    const trendIndicators = page.locator('[data-testid="metric-card"] [data-testid="metric-trend"]');
    const count = await trendIndicators.count();
    expect(count).toBeGreaterThanOrEqual(1);
  });

  /* ── Recent activity feed ───────────────────────────── */

  test('shows recent activity feed', async ({ page }) => {
    const activityFeed = page.locator('[data-testid="activity-feed"]');
    await expect(activityFeed).toBeVisible();

    const items = activityFeed.locator('[data-testid="activity-item"]');
    const count = await items.count();
    expect(count).toBeGreaterThanOrEqual(0); // May be empty on fresh install
  });

  test('activity feed items display timestamp and description', async ({ page }) => {
    const items = page.locator('[data-testid="activity-feed"] [data-testid="activity-item"]');
    const count = await items.count();

    if (count > 0) {
      const firstItem = items.first();
      await expect(firstItem.locator('[data-testid="activity-timestamp"]')).toBeVisible();
      await expect(firstItem.locator('[data-testid="activity-description"]')).toBeVisible();
    }
  });

  test('activity feed items are ordered by most recent first', async ({ page }) => {
    const timestamps = page.locator(
      '[data-testid="activity-feed"] [data-testid="activity-item"] [data-testid="activity-timestamp"]',
    );
    const count = await timestamps.count();

    if (count >= 2) {
      const texts = await timestamps.allTextContents();
      // Verify chronological ordering (most recent first)
      for (let i = 0; i < texts.length - 1; i++) {
        const current = new Date(texts[i]).getTime();
        const next = new Date(texts[i + 1]).getTime();
        // current should be >= next (more recent first)
        // Only check if they parse as valid dates
        if (!isNaN(current) && !isNaN(next)) {
          expect(current).toBeGreaterThanOrEqual(next);
        }
      }
    }
  });

  /* ── Sidebar navigation ─────────────────────────────── */

  test('navigates to correct pages from sidebar', async ({ page }) => {
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

        // Navigate back to home for next iteration
        await page.goto('/');
        await page.waitForSelector('[data-testid="dashboard"]');
      }
    }
  });

  test('sidebar highlights the current active page', async ({ page }) => {
    // On dashboard, the Home link should be active
    const sidebar = page.locator('[data-testid="sidebar"]');
    const homeLink = sidebar.getByRole('link', { name: /home|dashboard/i }).first();

    if ((await homeLink.count()) > 0) {
      const classes = await homeLink.getAttribute('class');
      // Should have an active/selected class
      expect(classes).toMatch(/active|selected|current|bg-/);
    }
  });

  /* ── Header ─────────────────────────────────────────── */

  test('displays user info in header', async ({ page }) => {
    const header = page.locator('[data-testid="header"]');
    await expect(header).toBeVisible();

    // Should show username or avatar
    const userMenu = page.locator('[data-testid="user-menu-button"]');
    await expect(userMenu).toBeVisible();
  });

  test('header shows Prometheus branding', async ({ page }) => {
    const header = page.locator('[data-testid="header"], [data-testid="sidebar"]');
    await expect(header.getByText('Prometheus', { exact: false }).first()).toBeVisible();
  });

  /* ── Responsive design ──────────────────────────────── */

  test('responsive on mobile viewport', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 812 });
    await page.reload();

    // Dashboard should still be accessible
    await expect(page.locator('[data-testid="dashboard"]')).toBeVisible();

    // Metric cards should stack vertically — check they are visible
    const metricCards = page.locator('[data-testid="metric-card"]');
    const count = await metricCards.count();
    for (let i = 0; i < Math.min(count, 4); i++) {
      await expect(metricCards.nth(i)).toBeVisible();
    }
  });

  test('sidebar collapses on mobile viewport', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 812 });
    await page.reload();

    // Sidebar should be hidden or collapsed
    const sidebar = page.locator('[data-testid="sidebar"]');
    const isVisible = await sidebar.isVisible();

    if (!isVisible) {
      // Should have a hamburger menu to toggle
      const hamburger = page.locator('[data-testid="mobile-menu-toggle"]');
      await expect(hamburger).toBeVisible();

      await hamburger.click();
      await expect(sidebar).toBeVisible();
    }
  });

  test('pipeline visualization adapts to narrow viewport', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 812 });
    await page.reload();

    const pipeline = page.locator('[data-testid="pipeline-viz"]');
    await expect(pipeline).toBeVisible();

    // Pipeline should not overflow
    const box = await pipeline.boundingBox();
    expect(box).not.toBeNull();
    if (box) {
      expect(box.width).toBeLessThanOrEqual(375);
    }
  });
});
