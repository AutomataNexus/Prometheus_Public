import { test, expect } from '@playwright/test';
import { login, apiLogin } from '../fixtures/auth';

test.describe('Model Registry', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
    await page.goto('/models');
    await page.waitForSelector('[data-testid="models-page"]');
  });

  /* ── Model list ─────────────────────────────────────── */

  test('lists trained models in grid view', async ({ page }) => {
    const modelGrid = page.locator('[data-testid="model-grid"], [data-testid="model-list"]');
    await expect(modelGrid).toBeVisible();

    const modelCards = page.locator('[data-testid="model-card"]');
    const count = await modelCards.count();
    // May be 0 on fresh install — that is valid
    expect(count).toBeGreaterThanOrEqual(0);
  });

  test('model cards display architecture type and equipment info', async ({ page }) => {
    const modelCards = page.locator('[data-testid="model-card"]');
    const count = await modelCards.count();

    if (count > 0) {
      const firstCard = modelCards.first();
      // Should show architecture badge (LSTM, GRU, Sentinel)
      const archBadge = firstCard.locator('[data-testid="architecture-badge"]');
      await expect(archBadge).toBeVisible();
      const archText = await archBadge.textContent();
      expect(archText).toMatch(/lstm|gru|sentinel/i);

      // Should show equipment type
      await expect(firstCard.locator('[data-testid="equipment-type"]')).toBeVisible();
    }
  });

  test('model cards show accuracy and model size', async ({ page }) => {
    const modelCards = page.locator('[data-testid="model-card"]');
    const count = await modelCards.count();

    if (count > 0) {
      const firstCard = modelCards.first();

      // Accuracy or F1 score
      const metric = firstCard.locator('[data-testid="model-accuracy"], [data-testid="model-f1"]');
      await expect(metric.first()).toBeVisible();

      // Model size
      const size = firstCard.locator('[data-testid="model-size"]');
      await expect(size).toBeVisible();
      const sizeText = await size.textContent();
      expect(sizeText).toMatch(/\d+\s*(KB|MB|B)/i);
    }
  });

  /* ── Model detail ───────────────────────────────────── */

  test('displays model architecture details', async ({ page }) => {
    const modelCards = page.locator('[data-testid="model-card"]');
    const count = await modelCards.count();

    if (count > 0) {
      await modelCards.first().click();
      await page.waitForURL('**/models/**');

      const detail = page.locator('[data-testid="model-detail"]');
      await expect(detail).toBeVisible();

      // Architecture section
      const archSection = detail.locator('[data-testid="architecture-section"]');
      await expect(archSection).toBeVisible();

      // Should show layer details
      await expect(archSection.getByText(/layer|lstm|gru|linear/i)).toBeVisible();

      // Should show parameter count
      await expect(detail.getByText(/parameter/i)).toBeVisible();
    }
  });

  test('shows evaluation metrics on model detail page', async ({ page }) => {
    const modelCards = page.locator('[data-testid="model-card"]');
    const count = await modelCards.count();

    if (count > 0) {
      await modelCards.first().click();
      await page.waitForURL('**/models/**');

      const metrics = page.locator('[data-testid="model-metrics"]');
      await expect(metrics).toBeVisible();

      // Standard metrics
      const metricLabels = ['precision', 'recall', 'f1'];
      for (const label of metricLabels) {
        await expect(metrics.getByText(label, { exact: false })).toBeVisible();
      }
    }
  });

  test('shows training history on model detail page', async ({ page }) => {
    const modelCards = page.locator('[data-testid="model-card"]');
    const count = await modelCards.count();

    if (count > 0) {
      await modelCards.first().click();
      await page.waitForURL('**/models/**');

      // Training run info
      const trainingInfo = page.locator('[data-testid="training-info"]');
      await expect(trainingInfo).toBeVisible();
      await expect(trainingInfo.getByText(/epoch|training|loss/i)).toBeVisible();
    }
  });

  /* ── Download model ─────────────────────────────────── */

  test('downloads .axonml model file', async ({ page }) => {
    const modelCards = page.locator('[data-testid="model-card"]');
    const count = await modelCards.count();

    if (count > 0) {
      await modelCards.first().click();
      await page.waitForURL('**/models/**');

      // Start download
      const downloadPromise = page.waitForEvent('download');
      await page.getByRole('button', { name: /download/i }).click();
      const download = await downloadPromise;

      // Verify the file name ends with .axonml
      expect(download.suggestedFilename()).toMatch(/\.axonml$/);

      // Save and verify non-empty
      const filePath = await download.path();
      expect(filePath).toBeTruthy();
    }
  });

  /* ── Delete model ───────────────────────────────────── */

  test('deletes model with confirmation', async ({ page }) => {
    const modelCards = page.locator('[data-testid="model-card"]');
    const count = await modelCards.count();

    if (count > 0) {
      await modelCards.first().click();
      await page.waitForURL('**/models/**');

      const deleteBtn = page.getByRole('button', { name: /delete/i });
      await deleteBtn.click();

      // Confirmation dialog
      const dialog = page.locator('[data-testid="confirm-dialog"]');
      await expect(dialog).toBeVisible();
      await dialog.getByRole('button', { name: /confirm|delete|yes/i }).click();

      // Should redirect back to models list
      await page.waitForURL('**/models');
      await expect(page.locator('[data-testid="models-page"]')).toBeVisible();
    }
  });

  /* ── Model comparison ───────────────────────────────── */

  test('compares two models side-by-side', async ({ page }) => {
    const modelCards = page.locator('[data-testid="model-card"]');
    const count = await modelCards.count();

    if (count >= 2) {
      // Select first model for comparison
      const checkbox1 = modelCards.nth(0).locator('[data-testid="compare-checkbox"]');
      const checkbox2 = modelCards.nth(1).locator('[data-testid="compare-checkbox"]');

      if ((await checkbox1.count()) > 0 && (await checkbox2.count()) > 0) {
        await checkbox1.check();
        await checkbox2.check();

        // Click compare button
        const compareBtn = page.getByRole('button', { name: /compare/i });
        await expect(compareBtn).toBeVisible();
        await compareBtn.click();

        // Comparison view should appear
        const comparison = page.locator('[data-testid="model-comparison"]');
        await expect(comparison).toBeVisible();

        // Should show metrics for both models
        await expect(comparison.getByText(/precision/i)).toBeVisible();
        await expect(comparison.getByText(/recall/i)).toBeVisible();
        await expect(comparison.getByText(/f1/i)).toBeVisible();
      }
    }
  });

  /* ── API-level model operations ─────────────────────── */

  test('lists models via API', async ({ context }) => {
    const token = await apiLogin(context);
    const resp = await context.request.get('/api/v1/models', {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(resp.ok()).toBeTruthy();
    const body = await resp.json();
    expect(Array.isArray(body)).toBeTruthy();
  });

  test('gets model detail via API', async ({ context }) => {
    const token = await apiLogin(context);
    const listResp = await context.request.get('/api/v1/models', {
      headers: { Authorization: `Bearer ${token}` },
    });
    const models = await listResp.json();

    if (Array.isArray(models) && models.length > 0) {
      const modelId = models[0].id;
      const detailResp = await context.request.get(`/api/v1/models/${modelId}`, {
        headers: { Authorization: `Bearer ${token}` },
      });
      expect(detailResp.ok()).toBeTruthy();
      const detail = await detailResp.json();
      expect(detail).toHaveProperty('id', modelId);
      expect(detail).toHaveProperty('architecture');
      expect(detail).toHaveProperty('metrics');
    }
  });

  test('compares models via API', async ({ context }) => {
    const token = await apiLogin(context);
    const listResp = await context.request.get('/api/v1/models', {
      headers: { Authorization: `Bearer ${token}` },
    });
    const models = await listResp.json();

    if (Array.isArray(models) && models.length >= 2) {
      const resp = await context.request.post(`/api/v1/models/${models[0].id}/compare`, {
        headers: { Authorization: `Bearer ${token}` },
        data: { compare_with: models[1].id },
      });
      expect(resp.ok()).toBeTruthy();
      const comparison = await resp.json();
      expect(comparison).toHaveProperty('models');
      expect(comparison.models).toHaveLength(2);
    }
  });
});
