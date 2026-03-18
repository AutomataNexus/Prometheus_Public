import { test, expect } from '@playwright/test';
import { login, apiLogin } from '../fixtures/auth';

test.describe('Model Evaluation', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
    await page.goto('/evaluation');
    await page.waitForSelector('[data-testid="evaluation-page"]');
  });

  /* ── Metrics dashboard ──────────────────────────────── */

  test('displays accuracy, precision, recall, F1 metrics', async ({ page }) => {
    const evaluations = page.locator('[data-testid="evaluation-card"]');
    const count = await evaluations.count();

    if (count > 0) {
      await evaluations.first().click();

      const metricsPanel = page.locator('[data-testid="metrics-panel"]');
      await expect(metricsPanel).toBeVisible();

      const expectedMetrics = ['accuracy', 'precision', 'recall', 'f1'];
      for (const metric of expectedMetrics) {
        const metricEl = metricsPanel.getByText(metric, { exact: false });
        await expect(metricEl.first()).toBeVisible();
      }
    }
  });

  test('metric values are between 0 and 1', async ({ page }) => {
    const evaluations = page.locator('[data-testid="evaluation-card"]');
    const count = await evaluations.count();

    if (count > 0) {
      await evaluations.first().click();

      const metricValues = page.locator('[data-testid="metrics-panel"] [data-testid="metric-value"]');
      const valCount = await metricValues.count();

      for (let i = 0; i < valCount; i++) {
        const text = await metricValues.nth(i).textContent();
        const value = parseFloat(text?.replace('%', '') ?? '0');
        // Values could be 0-1 (ratio) or 0-100 (percentage)
        if (text?.includes('%')) {
          expect(value).toBeGreaterThanOrEqual(0);
          expect(value).toBeLessThanOrEqual(100);
        } else {
          expect(value).toBeGreaterThanOrEqual(0);
          expect(value).toBeLessThanOrEqual(1);
        }
      }
    }
  });

  test('displays AUC-ROC metric', async ({ page }) => {
    const evaluations = page.locator('[data-testid="evaluation-card"]');
    const count = await evaluations.count();

    if (count > 0) {
      await evaluations.first().click();

      const auc = page.locator('[data-testid="metrics-panel"]').getByText(/auc|roc/i);
      await expect(auc.first()).toBeVisible();
    }
  });

  /* ── Training curves chart ──────────────────────────── */

  test('renders training curves chart with loss over epochs', async ({ page }) => {
    const evaluations = page.locator('[data-testid="evaluation-card"]');
    const count = await evaluations.count();

    if (count > 0) {
      await evaluations.first().click();

      const curvesChart = page.locator('[data-testid="training-curves"]');
      await expect(curvesChart).toBeVisible();

      // Chart should render SVG or canvas
      const chartEl = curvesChart.locator('svg, canvas');
      await expect(chartEl.first()).toBeVisible();
    }
  });

  test('training curves show both train and validation loss', async ({ page }) => {
    const evaluations = page.locator('[data-testid="evaluation-card"]');
    const count = await evaluations.count();

    if (count > 0) {
      await evaluations.first().click();

      const legend = page.locator('[data-testid="training-curves"] [data-testid="chart-legend"]');
      if ((await legend.count()) > 0) {
        await expect(legend.getByText(/train/i)).toBeVisible();
        await expect(legend.getByText(/val/i)).toBeVisible();
      }
    }
  });

  /* ── Confusion matrix ───────────────────────────────── */

  test('shows confusion matrix heatmap for classifier models', async ({ page }) => {
    const evaluations = page.locator('[data-testid="evaluation-card"]');
    const count = await evaluations.count();

    if (count > 0) {
      await evaluations.first().click();

      const confusionMatrix = page.locator('[data-testid="confusion-matrix"]');

      // Confusion matrix may not be present for all model types (e.g., autoencoders)
      if ((await confusionMatrix.count()) > 0) {
        await expect(confusionMatrix).toBeVisible();

        // Should have cells
        const cells = confusionMatrix.locator('[data-testid="matrix-cell"]');
        const cellCount = await cells.count();
        expect(cellCount).toBeGreaterThanOrEqual(4); // At least 2x2

        // Cells should have numeric content
        for (let i = 0; i < cellCount; i++) {
          const text = await cells.nth(i).textContent();
          expect(text?.trim()).toMatch(/\d+/);
        }
      }
    }
  });

  test('confusion matrix labels show true/predicted axes', async ({ page }) => {
    const evaluations = page.locator('[data-testid="evaluation-card"]');
    const count = await evaluations.count();

    if (count > 0) {
      await evaluations.first().click();

      const confusionMatrix = page.locator('[data-testid="confusion-matrix"]');

      if ((await confusionMatrix.count()) > 0) {
        // Should have axis labels
        await expect(confusionMatrix.getByText(/true|actual/i)).toBeVisible();
        await expect(confusionMatrix.getByText(/predicted/i)).toBeVisible();
      }
    }
  });

  /* ── Gradient AI evaluation ─────────────────────────── */

  test('triggers Gradient AI evaluation and displays results', async ({ page }) => {
    const evaluations = page.locator('[data-testid="evaluation-card"]');
    const count = await evaluations.count();

    if (count > 0) {
      await evaluations.first().click();

      const gradientSection = page.locator('[data-testid="gradient-evaluation"]');
      await expect(gradientSection).toBeVisible();

      // Click "Run Gradient Evaluation" if not already run
      const runBtn = gradientSection.getByRole('button', { name: /run|evaluate|gradient/i });
      if ((await runBtn.count()) > 0 && (await runBtn.isEnabled())) {
        await runBtn.click();

        // Wait for evaluation to complete
        await expect(
          gradientSection.locator('[data-testid="gradient-metrics"]'),
        ).toBeVisible({ timeout: 60_000 });
      }

      // Gradient metrics should be displayed
      const gradientMetrics = gradientSection.locator('[data-testid="gradient-metrics"]');
      if ((await gradientMetrics.count()) > 0) {
        await expect(gradientMetrics).toBeVisible();

        // Should show multiple evaluation metrics
        const metricRows = gradientMetrics.locator('[data-testid="gradient-metric-row"]');
        const metricCount = await metricRows.count();
        expect(metricCount).toBeGreaterThanOrEqual(1);
      }
    }
  });

  test('displays Gradient evaluation score summary', async ({ page }) => {
    const evaluations = page.locator('[data-testid="evaluation-card"]');
    const count = await evaluations.count();

    if (count > 0) {
      await evaluations.first().click();

      const gradientSection = page.locator('[data-testid="gradient-evaluation"]');
      const scoreSummary = gradientSection.locator('[data-testid="gradient-score-summary"]');

      if ((await scoreSummary.count()) > 0) {
        await expect(scoreSummary).toBeVisible();

        // Should show pass/fail or score
        const scoreText = await scoreSummary.textContent();
        expect(scoreText).toMatch(/pass|fail|score|\d+/i);
      }
    }
  });

  /* ── Evaluation list ────────────────────────────────── */

  test('evaluation list shows model name and date', async ({ page }) => {
    const evaluations = page.locator('[data-testid="evaluation-card"]');
    const count = await evaluations.count();

    if (count > 0) {
      const firstCard = evaluations.first();
      // Model name
      await expect(firstCard.locator('[data-testid="eval-model-name"]')).toBeVisible();
      // Date
      await expect(firstCard.locator('[data-testid="eval-date"]')).toBeVisible();
    }
  });

  /* ── API-level evaluation ───────────────────────────── */

  test('lists evaluations via API', async ({ context }) => {
    const token = await apiLogin(context);
    const resp = await context.request.get('/api/v1/evaluations', {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(resp.ok()).toBeTruthy();
    const body = await resp.json();
    expect(Array.isArray(body)).toBeTruthy();
  });

  test('gets evaluation detail via API', async ({ context }) => {
    const token = await apiLogin(context);
    const listResp = await context.request.get('/api/v1/evaluations', {
      headers: { Authorization: `Bearer ${token}` },
    });
    const evals = await listResp.json();

    if (Array.isArray(evals) && evals.length > 0) {
      const evalId = evals[0].id;
      const detailResp = await context.request.get(`/api/v1/evaluations/${evalId}`, {
        headers: { Authorization: `Bearer ${token}` },
      });
      expect(detailResp.ok()).toBeTruthy();
      const detail = await detailResp.json();
      expect(detail).toHaveProperty('id', evalId);
      expect(detail).toHaveProperty('metrics');
    }
  });

  test('triggers Gradient evaluation via API', async ({ context }) => {
    const token = await apiLogin(context);
    const listResp = await context.request.get('/api/v1/evaluations', {
      headers: { Authorization: `Bearer ${token}` },
    });
    const evals = await listResp.json();

    if (Array.isArray(evals) && evals.length > 0) {
      const evalId = evals[0].id;
      const gradientResp = await context.request.post(
        `/api/v1/evaluations/${evalId}/gradient`,
        {
          headers: { Authorization: `Bearer ${token}` },
        },
      );
      // May return 200 or 202 (accepted)
      expect([200, 202]).toContain(gradientResp.status());
    }
  });
});
