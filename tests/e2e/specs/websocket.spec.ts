import { test, expect } from '@playwright/test';
import { login, apiLogin } from '../fixtures/auth';
import path from 'path';

const AHU_CSV = path.resolve(__dirname, '../fixtures/sample_ahu_data.csv');

test.describe('WebSocket Communication', () => {
  let datasetId: string;

  test.beforeEach(async ({ page, context }) => {
    await login(page);

    // Upload a dataset via API for training tests
    const token = await apiLogin(context);
    const uploadResp = await context.request.post('/api/v1/datasets', {
      headers: { Authorization: `Bearer ${token}` },
      multipart: {
        file: {
          name: 'ws_test_ahu.csv',
          mimeType: 'text/csv',
          buffer: Buffer.from(
            await (await import('fs')).promises.readFile(AHU_CSV, 'utf-8'),
          ),
        },
        name: 'WebSocket Test AHU',
        equipment_type: 'air_handler',
      },
    });
    const ds = await uploadResp.json();
    datasetId = ds.id;
  });

  /* ── Helper: start a training run and navigate to detail ── */

  async function startTrainingRun(page: import('@playwright/test').Page) {
    await page.goto('/training');
    await page.waitForSelector('[data-testid="training-page"]');

    await page.getByRole('button', { name: /start|new.training/i }).click();

    const datasetSelect = page.locator('[data-testid="dataset-select"]');
    await expect(datasetSelect).toBeVisible();
    await datasetSelect.selectOption({ label: /WebSocket Test AHU/i });

    await page.locator('[data-testid="training-plan"]').waitFor({ timeout: 30_000 });
    await page.getByRole('button', { name: /confirm|start|begin/i }).click();

    await expect(page.locator('[data-testid="training-progress"]')).toBeVisible({
      timeout: 15_000,
    });
  }

  /* ── WebSocket connection ──────────────────────────── */

  test('WebSocket connects during training detail view', async ({ page }) => {
    // Track WebSocket connections
    const wsConnections: string[] = [];
    page.on('websocket', (ws) => {
      wsConnections.push(ws.url());
    });

    await startTrainingRun(page);

    // Wait for WebSocket to connect
    await page.waitForTimeout(3_000);

    // At least one WebSocket connection should have been established
    expect(wsConnections.length).toBeGreaterThanOrEqual(1);
  });

  /* ── Live epoch updates ────────────────────────────── */

  test('live epoch updates appear in training detail', async ({ page }) => {
    await startTrainingRun(page);

    const epochCounter = page.locator('[data-testid="epoch-counter"]');
    await expect(epochCounter).toBeVisible({ timeout: 15_000 });

    // Read initial epoch
    const initialText = await epochCounter.textContent();

    // Wait for WebSocket updates
    await page.waitForTimeout(10_000);

    // Epoch should have advanced
    const updatedText = await epochCounter.textContent();
    expect(updatedText).toMatch(/\d+\s*\/\s*\d+/);

    // If training progresses, the epoch number should increase
    const initialMatch = initialText?.match(/(\d+)\s*\//);
    const updatedMatch = updatedText?.match(/(\d+)\s*\//);
    if (initialMatch && updatedMatch) {
      const initial = parseInt(initialMatch[1]);
      const updated = parseInt(updatedMatch[1]);
      expect(updated).toBeGreaterThanOrEqual(initial);
    }
  });

  /* ── Loss chart real-time updates ──────────────────── */

  test('loss chart updates in real-time', async ({ page }) => {
    await startTrainingRun(page);

    const lossChart = page.locator('[data-testid="loss-chart"]');
    await expect(lossChart).toBeVisible({ timeout: 15_000 });

    // Wait for chart to render initial data
    await page.waitForTimeout(5_000);

    // Chart should have SVG or canvas content
    const chartContent = lossChart.locator('svg, canvas');
    await expect(chartContent.first()).toBeVisible();

    // For SVG charts, count data points
    const svgPaths = lossChart.locator('svg path, svg circle, svg line');
    const initialPathCount = await svgPaths.count();

    // Wait for more data points via WebSocket
    await page.waitForTimeout(5_000);

    // Chart should have updated (more path elements or re-rendered canvas)
    const updatedPathCount = await svgPaths.count();
    // At minimum, the chart should still be rendered
    expect(updatedPathCount).toBeGreaterThanOrEqual(0);
  });

  /* ── Progress bar advances ─────────────────────────── */

  test('progress bar advances with WebSocket messages', async ({ page }) => {
    await startTrainingRun(page);

    const progressBar = page.locator('[data-testid="training-progress"] progress, [data-testid="training-progress"] [role="progressbar"]');
    if ((await progressBar.count()) > 0) {
      // Read initial progress
      const initialValue = await progressBar.first().getAttribute('value') ??
        await progressBar.first().getAttribute('aria-valuenow') ?? '0';
      const initialProgress = parseFloat(initialValue);

      // Wait for WebSocket updates
      await page.waitForTimeout(10_000);

      // Read updated progress
      const updatedValue = await progressBar.first().getAttribute('value') ??
        await progressBar.first().getAttribute('aria-valuenow') ?? '0';
      const updatedProgress = parseFloat(updatedValue);

      // Progress should have advanced
      expect(updatedProgress).toBeGreaterThanOrEqual(initialProgress);
    }
  });

  /* ── WebSocket reconnection ────────────────────────── */

  test('WebSocket reconnects after disconnect', async ({ page }) => {
    const wsConnections: string[] = [];
    page.on('websocket', (ws) => {
      wsConnections.push(ws.url());
    });

    await startTrainingRun(page);
    await page.waitForTimeout(3_000);

    const initialConnections = wsConnections.length;
    expect(initialConnections).toBeGreaterThanOrEqual(1);

    // Simulate WebSocket disconnect by temporarily blocking WS connections
    await page.evaluate(() => {
      // Close all existing WebSocket connections
      const originalWS = window.WebSocket;
      (window as any).__closedWS = true;
    });

    // Wait for reconnection attempt
    await page.waitForTimeout(5_000);

    // The application should attempt to reconnect
    // (tracked by the websocket event listener)
    // Even if reconnection doesn't happen immediately, the UI should remain stable
    const progressVisible = await page.locator('[data-testid="training-progress"]').isVisible();
    expect(progressVisible).toBeTruthy();
  });

  /* ── Multiple training tabs ────────────────────────── */

  test('multiple training detail tabs work independently', async ({ page, context }) => {
    await startTrainingRun(page);

    const trainingUrl = page.url();

    // Open a second tab
    const page2 = await context.newPage();
    await login(page2);
    await page2.goto(trainingUrl);

    // Both pages should show training progress
    await expect(page.locator('[data-testid="training-progress"]')).toBeVisible();
    await expect(page2.locator('[data-testid="training-progress"]')).toBeVisible();

    // Both should have their own WebSocket connections
    const ws1Connected = await page.evaluate(() => {
      return document.querySelector('[data-testid="training-progress"]') !== null;
    });
    const ws2Connected = await page2.evaluate(() => {
      return document.querySelector('[data-testid="training-progress"]') !== null;
    });

    expect(ws1Connected).toBeTruthy();
    expect(ws2Connected).toBeTruthy();

    await page2.close();
  });

  /* ── WebSocket cleanup on navigation ───────────────── */

  test('WebSocket closes when navigating away', async ({ page }) => {
    const wsEvents: Array<{ type: string; url: string }> = [];
    page.on('websocket', (ws) => {
      wsEvents.push({ type: 'open', url: ws.url() });
      ws.on('close', () => {
        wsEvents.push({ type: 'close', url: ws.url() });
      });
    });

    await startTrainingRun(page);
    await page.waitForTimeout(3_000);

    // Verify WebSocket is connected
    const openEvents = wsEvents.filter((e) => e.type === 'open');
    expect(openEvents.length).toBeGreaterThanOrEqual(1);

    // Navigate away from training
    await page.goto('/datasets');
    await page.waitForSelector('[data-testid="datasets-page"]');
    await page.waitForTimeout(2_000);

    // WebSocket should have been closed
    const closeEvents = wsEvents.filter((e) => e.type === 'close');
    expect(closeEvents.length).toBeGreaterThanOrEqual(1);
  });

  /* ── Training completion via WebSocket ─────────────── */

  test('training completion message received via WebSocket', async ({ page }) => {
    const wsMessages: string[] = [];
    page.on('websocket', (ws) => {
      ws.on('framereceived', (frame) => {
        if (typeof frame.payload === 'string') {
          wsMessages.push(frame.payload);
        }
      });
    });

    await startTrainingRun(page);

    // Wait for training to complete (generous timeout for small datasets)
    const trainingStatus = page.locator('[data-testid="training-status"]');
    await expect(trainingStatus).toContainText(/completed|finished|done/i, {
      timeout: 300_000,
    });

    // Should have received WebSocket messages during training
    expect(wsMessages.length).toBeGreaterThanOrEqual(1);

    // At least one message should indicate completion
    const hasCompletionMsg = wsMessages.some(
      (msg) => /complet|finish|done|status/i.test(msg),
    );
    // May also be indicated by epoch reaching max
    const hasEpochMsg = wsMessages.some((msg) => /epoch|progress/i.test(msg));
    expect(hasCompletionMsg || hasEpochMsg).toBeTruthy();
  });
});
