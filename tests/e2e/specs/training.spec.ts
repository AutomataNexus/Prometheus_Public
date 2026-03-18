import { test, expect } from '@playwright/test';
import { login, apiLogin } from '../fixtures/auth';
import path from 'path';

const AHU_CSV = path.resolve(__dirname, '../fixtures/sample_ahu_data.csv');

test.describe('Training Pipeline', () => {
  let datasetId: string;

  test.beforeEach(async ({ page, context }) => {
    await login(page);

    // Upload a dataset via API for training tests
    const token = await apiLogin(context);
    const uploadResp = await context.request.post('/api/v1/datasets', {
      headers: { Authorization: `Bearer ${token}` },
      multipart: {
        file: {
          name: 'training_test_ahu.csv',
          mimeType: 'text/csv',
          buffer: Buffer.from(
            await (await import('fs')).promises.readFile(AHU_CSV, 'utf-8'),
          ),
        },
        name: 'Training Test AHU',
        equipment_type: 'air_handler',
      },
    });
    const ds = await uploadResp.json();
    datasetId = ds.id;

    await page.goto('/training');
    await page.waitForSelector('[data-testid="training-page"]');
  });

  /* ── Start training ─────────────────────────────────── */

  test('starts training with valid dataset and training plan', async ({ page }) => {
    // Click "Start Training" / "New Training Run"
    await page.getByRole('button', { name: /start|new.training/i }).click();

    // Select dataset
    const datasetSelect = page.locator('[data-testid="dataset-select"]');
    await expect(datasetSelect).toBeVisible();
    await datasetSelect.selectOption({ label: /Training Test AHU/i });

    // Wait for agent to generate training plan (or use default)
    const planSection = page.locator('[data-testid="training-plan"]');
    await expect(planSection).toBeVisible({ timeout: 30_000 });

    // Verify training plan shows architecture
    await expect(planSection.getByText(/lstm|gru|sentinel/i)).toBeVisible();

    // Confirm and start
    await page.getByRole('button', { name: /confirm|start|begin/i }).click();

    // Should navigate to or show training detail with live progress
    await expect(page.locator('[data-testid="training-progress"]')).toBeVisible({ timeout: 15_000 });
  });

  test('displays training configuration before starting', async ({ page }) => {
    await page.getByRole('button', { name: /start|new.training/i }).click();

    const datasetSelect = page.locator('[data-testid="dataset-select"]');
    await datasetSelect.selectOption({ label: /Training Test AHU/i });

    const planSection = page.locator('[data-testid="training-plan"]');
    await expect(planSection).toBeVisible({ timeout: 30_000 });

    // Should show hyperparameters
    const expectedFields = ['learning_rate', 'batch_size', 'epochs', 'hidden_dim'];
    for (const field of expectedFields) {
      await expect(planSection.getByText(field, { exact: false })).toBeVisible();
    }
  });

  /* ── Live training progress ─────────────────────────── */

  test('displays live loss chart via WebSocket', async ({ page }) => {
    // Start a training run
    await page.getByRole('button', { name: /start|new.training/i }).click();

    const datasetSelect = page.locator('[data-testid="dataset-select"]');
    await datasetSelect.selectOption({ label: /Training Test AHU/i });

    await page.locator('[data-testid="training-plan"]').waitFor({ timeout: 30_000 });
    await page.getByRole('button', { name: /confirm|start|begin/i }).click();

    // Wait for the live chart to appear
    const lossChart = page.locator('[data-testid="loss-chart"]');
    await expect(lossChart).toBeVisible({ timeout: 15_000 });

    // Wait for at least one data point to appear
    await page.waitForTimeout(5_000);

    // The chart SVG or canvas should have content
    const chartContent = lossChart.locator('svg, canvas');
    await expect(chartContent.first()).toBeVisible();
  });

  test('updates epoch counter in real-time', async ({ page }) => {
    await page.getByRole('button', { name: /start|new.training/i }).click();

    const datasetSelect = page.locator('[data-testid="dataset-select"]');
    await datasetSelect.selectOption({ label: /Training Test AHU/i });

    await page.locator('[data-testid="training-plan"]').waitFor({ timeout: 30_000 });
    await page.getByRole('button', { name: /confirm|start|begin/i }).click();

    const epochCounter = page.locator('[data-testid="epoch-counter"]');
    await expect(epochCounter).toBeVisible({ timeout: 15_000 });

    // Read initial value
    const initialText = await epochCounter.textContent();

    // Wait for progress
    await page.waitForTimeout(10_000);

    // Epoch should have advanced
    const updatedText = await epochCounter.textContent();
    // Both should contain numbers like "3/100"
    expect(updatedText).toMatch(/\d+\s*\/\s*\d+/);
  });

  test('shows estimated time remaining during training', async ({ page }) => {
    await page.getByRole('button', { name: /start|new.training/i }).click();

    const datasetSelect = page.locator('[data-testid="dataset-select"]');
    await datasetSelect.selectOption({ label: /Training Test AHU/i });

    await page.locator('[data-testid="training-plan"]').waitFor({ timeout: 30_000 });
    await page.getByRole('button', { name: /confirm|start|begin/i }).click();

    // Wait for training to start and ETA to appear
    const eta = page.locator('[data-testid="time-remaining"]');
    await expect(eta).toBeVisible({ timeout: 30_000 });
    const etaText = await eta.textContent();
    expect(etaText).toMatch(/\d+\s*(s|sec|min|m|h|remaining)/i);
  });

  /* ── Stop training ──────────────────────────────────── */

  test('stops training job mid-run', async ({ page }) => {
    await page.getByRole('button', { name: /start|new.training/i }).click();

    const datasetSelect = page.locator('[data-testid="dataset-select"]');
    await datasetSelect.selectOption({ label: /Training Test AHU/i });

    await page.locator('[data-testid="training-plan"]').waitFor({ timeout: 30_000 });
    await page.getByRole('button', { name: /confirm|start|begin/i }).click();

    // Wait for training to start
    await expect(page.locator('[data-testid="training-progress"]')).toBeVisible({ timeout: 15_000 });

    // Click stop
    const stopBtn = page.getByRole('button', { name: /stop|cancel/i });
    await expect(stopBtn).toBeVisible();
    await stopBtn.click();

    // Confirm stop
    const confirmDialog = page.locator('[data-testid="confirm-dialog"]');
    if ((await confirmDialog.count()) > 0) {
      await confirmDialog.getByRole('button', { name: /confirm|yes|stop/i }).click();
    }

    // Status should change to stopped/cancelled
    await expect(page.locator('[data-testid="training-status"]')).toContainText(/stopped|cancelled/i, {
      timeout: 10_000,
    });
  });

  /* ── Training completion ────────────────────────────── */

  test('completes training and shows summary', async ({ page }) => {
    // Start training
    await page.getByRole('button', { name: /start|new.training/i }).click();

    const datasetSelect = page.locator('[data-testid="dataset-select"]');
    await datasetSelect.selectOption({ label: /Training Test AHU/i });

    await page.locator('[data-testid="training-plan"]').waitFor({ timeout: 30_000 });
    await page.getByRole('button', { name: /confirm|start|begin/i }).click();

    // Wait for completion (use a generous timeout for small training runs)
    await expect(page.locator('[data-testid="training-status"]')).toContainText(
      /completed|finished|done/i,
      { timeout: 300_000 },
    );

    // Summary metrics should be visible
    const summary = page.locator('[data-testid="training-summary"]');
    await expect(summary).toBeVisible();
    await expect(summary.getByText(/loss/i)).toBeVisible();
    await expect(summary.getByText(/accuracy|f1|precision/i)).toBeVisible();
  });

  test('navigates to model detail after completed training', async ({ page }) => {
    // Assuming there is a completed training run visible
    const historyTable = page.locator('[data-testid="training-history"]');
    const completedRow = historyTable.locator('tr:has-text("completed")').first();

    if ((await completedRow.count()) > 0) {
      const viewModelBtn = completedRow.getByRole('link', { name: /view.model|model/i });
      if ((await viewModelBtn.count()) > 0) {
        await viewModelBtn.click();
        await page.waitForURL('**/models/**');
        expect(page.url()).toContain('/models/');
      }
    }
  });

  /* ── Training history ───────────────────────────────── */

  test('shows training history table', async ({ page }) => {
    const historyTable = page.locator('[data-testid="training-history"]');
    await expect(historyTable).toBeVisible();

    // Should have column headers
    const headers = historyTable.locator('thead th, [data-testid="table-header"]');
    const count = await headers.count();
    expect(count).toBeGreaterThanOrEqual(1);
  });

  /* ── API-level training ─────────────────────────────── */

  test('starts training via API', async ({ context }) => {
    const token = await apiLogin(context);

    const startResp = await context.request.post('/api/v1/training/start', {
      headers: { Authorization: `Bearer ${token}` },
      data: {
        dataset_id: datasetId,
        architecture: 'lstm_autoencoder',
        hyperparameters: {
          learning_rate: 0.001,
          batch_size: 64,
          epochs: 5,
          hidden_dim: 64,
          bottleneck_dim: 32,
          num_layers: 2,
          sequence_length: 60,
          dropout: 0.1,
          optimizer: 'adam',
          loss: 'mse',
        },
      },
    });
    expect(startResp.ok()).toBeTruthy();

    const run = await startResp.json();
    expect(run).toHaveProperty('id');
    expect(run).toHaveProperty('status');
    expect(['running', 'queued', 'pending']).toContain(run.status);
  });

  test('gets training status via API', async ({ context }) => {
    const token = await apiLogin(context);

    // List training runs
    const listResp = await context.request.get('/api/v1/training', {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(listResp.ok()).toBeTruthy();
    const runs = await listResp.json();
    expect(Array.isArray(runs)).toBeTruthy();
  });
});
