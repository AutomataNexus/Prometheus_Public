import { test, expect } from '@playwright/test';
import { login, apiLogin, apiRequest } from '../fixtures/auth';
import path from 'path';

const AHU_CSV = path.resolve(__dirname, '../fixtures/sample_ahu_data.csv');

test.describe('Pipeline Workflow — End-to-End', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  /* ── Full pipeline workflow ────────────────────────── */

  test('complete workflow: upload CSV, train, monitor, view model, deploy', async ({
    page,
    context,
  }) => {
    // Step 1: Upload CSV
    await page.goto('/datasets');
    await page.waitForSelector('[data-testid="datasets-page"]');

    const fileInput = page.locator('[data-testid="file-upload"] input[type="file"]');
    await fileInput.setInputFiles(AHU_CSV);
    await expect(page.locator('[data-testid="upload-success"]')).toBeVisible({
      timeout: 30_000,
    });

    // Verify dataset appears in list
    const table = page.locator('[data-testid="dataset-table"]');
    await expect(table).toBeVisible();
    await expect(table.getByText('sample_ahu_data', { exact: false })).toBeVisible();

    // Click dataset to see detail
    await page.locator('[data-testid="dataset-table"] tr').last().click();
    await page.waitForURL('**/datasets/**');
    const datasetUrl = page.url();
    const datasetIdMatch = datasetUrl.match(/datasets\/([^/]+)/);

    // Step 2: Navigate to training and start a run
    await page.goto('/training');
    await page.waitForSelector('[data-testid="training-page"]');

    await page.getByRole('button', { name: /start|new.training/i }).click();

    const datasetSelect = page.locator('[data-testid="dataset-select"]');
    await expect(datasetSelect).toBeVisible();
    await datasetSelect.selectOption({ label: /sample_ahu_data|ahu/i });

    await page.locator('[data-testid="training-plan"]').waitFor({ timeout: 30_000 });
    await page.getByRole('button', { name: /confirm|start|begin/i }).click();

    // Step 3: Monitor training progress
    await expect(page.locator('[data-testid="training-progress"]')).toBeVisible({
      timeout: 15_000,
    });

    const epochCounter = page.locator('[data-testid="epoch-counter"]');
    await expect(epochCounter).toBeVisible({ timeout: 15_000 });

    // Wait for completion
    await expect(page.locator('[data-testid="training-status"]')).toContainText(
      /completed|finished|done/i,
      { timeout: 300_000 },
    );

    // Step 4: View the trained model
    const viewModelLink = page.getByRole('link', { name: /view.model|model/i }).first();
    if ((await viewModelLink.count()) > 0) {
      await viewModelLink.click();
      await page.waitForURL('**/models/**');

      const modelDetail = page.locator('[data-testid="model-detail"]');
      await expect(modelDetail).toBeVisible();

      // Step 5: Deploy model
      const deployBtn = page.getByRole('button', { name: /deploy/i });
      if ((await deployBtn.count()) > 0) {
        await deployBtn.click();

        const targetSelect = page.locator('[data-testid="target-select"]');
        if ((await targetSelect.count()) > 0) {
          const targetOptions = targetSelect.locator('option');
          if ((await targetOptions.count()) > 1) {
            await targetSelect.selectOption({ index: 1 });
          }
          const confirmBtn = page.getByRole('button', { name: /confirm|deploy/i });
          if ((await confirmBtn.count()) > 0) {
            await confirmBtn.click();

            const deployStatus = page.locator('[data-testid="deployment-status"]');
            await expect(deployStatus).toBeVisible({ timeout: 30_000 });
          }
        }
      }
    }
  });

  /* ── Upload dataset and verify list ────────────────── */

  test('upload dataset and verify it appears in list', async ({ page }) => {
    await page.goto('/datasets');
    await page.waitForSelector('[data-testid="datasets-page"]');

    const fileInput = page.locator('[data-testid="file-upload"] input[type="file"]');
    await fileInput.setInputFiles(AHU_CSV);

    await expect(page.locator('[data-testid="upload-success"]')).toBeVisible({
      timeout: 30_000,
    });

    // Verify in list
    const table = page.locator('[data-testid="dataset-table"]');
    await expect(table).toBeVisible();

    const rows = table.locator('tbody tr');
    const count = await rows.count();
    expect(count).toBeGreaterThanOrEqual(1);

    // Newest upload should be visible
    await expect(table.getByText(/ahu|sample/i).first()).toBeVisible();
  });

  /* ── Start training from dataset detail ────────────── */

  test('start training from dataset detail page', async ({ page }) => {
    await page.goto('/datasets');
    await page.waitForSelector('[data-testid="datasets-page"]');

    // Upload a dataset first
    const fileInput = page.locator('[data-testid="file-upload"] input[type="file"]');
    await fileInput.setInputFiles(AHU_CSV);
    await expect(page.locator('[data-testid="upload-success"]')).toBeVisible({
      timeout: 30_000,
    });

    // Navigate to dataset detail
    await page.locator('[data-testid="dataset-table"] tr').last().click();
    await page.waitForURL('**/datasets/**');

    // Look for a "Train Model" or "Start Training" button on the detail page
    const trainBtn = page.getByRole('button', { name: /train|start.training/i });
    if ((await trainBtn.count()) > 0) {
      await trainBtn.click();

      // Should navigate to training page with dataset pre-selected
      await page.waitForURL('**/training**', { timeout: 10_000 });
      expect(page.url()).toContain('/training');
    } else {
      // Alternative: navigate manually to training
      await page.goto('/training');
      await page.waitForSelector('[data-testid="training-page"]');

      await page.getByRole('button', { name: /start|new.training/i }).click();
      const datasetSelect = page.locator('[data-testid="dataset-select"]');
      await expect(datasetSelect).toBeVisible();

      // Dataset should be available in the select
      const options = await datasetSelect.locator('option').allTextContents();
      const hasAhu = options.some((opt) => /ahu|sample/i.test(opt));
      expect(hasAhu).toBeTruthy();
    }
  });

  /* ── Navigate from training to model ───────────────── */

  test('navigate from training to model after completion', async ({ page }) => {
    await page.goto('/training');
    await page.waitForSelector('[data-testid="training-page"]');

    const historyTable = page.locator('[data-testid="training-history"]');
    const completedRow = historyTable.locator('tr:has-text("completed")').first();

    if ((await completedRow.count()) > 0) {
      const viewModelLink = completedRow.getByRole('link', { name: /view.model|model/i });
      if ((await viewModelLink.count()) > 0) {
        await viewModelLink.click();
        await page.waitForURL('**/models/**');

        const modelDetail = page.locator('[data-testid="model-detail"]');
        await expect(modelDetail).toBeVisible();

        // Verify model shows architecture info
        const archSection = modelDetail.locator('[data-testid="architecture-section"]');
        if ((await archSection.count()) > 0) {
          await expect(archSection).toBeVisible();
        }
      }
    }
  });

  /* ── Deploy model and verify status ────────────────── */

  test('deploy model and verify deployment status', async ({ page }) => {
    await page.goto('/deployment');
    await page.waitForSelector('[data-testid="deployment-page"]');

    const deployBtn = page.getByRole('button', { name: /deploy|new.deployment/i });
    await deployBtn.click();

    const modelSelect = page.locator('[data-testid="model-select"]');
    await expect(modelSelect).toBeVisible();

    const modelOptions = modelSelect.locator('option');
    if ((await modelOptions.count()) > 1) {
      await modelSelect.selectOption({ index: 1 });

      const targetSelect = page.locator('[data-testid="target-select"]');
      if ((await targetSelect.count()) > 0 && (await targetSelect.locator('option').count()) > 1) {
        await targetSelect.selectOption({ index: 1 });
      }

      await page.getByRole('button', { name: /confirm|deploy/i }).click();

      // Handle confirmation modal
      const confirmModal = page.locator('[data-testid="deploy-confirm-modal"]');
      if ((await confirmModal.count()) > 0) {
        await confirmModal.getByRole('button', { name: /confirm|yes|deploy/i }).click();
      }

      // Verify deployment status appears
      const statusBadge = page.locator('[data-testid="deployment-status"], [data-testid="deployment-status-badge"]');
      await expect(statusBadge.first()).toBeVisible({ timeout: 30_000 });

      const statusText = await statusBadge.first().textContent();
      expect(statusText).toMatch(/pending|compiling|deploying|deployed/i);
    }
  });

  /* ── Agent recommends architecture ─────────────────── */

  test('agent recommends architecture for uploaded dataset', async ({ page, context }) => {
    // Upload a dataset via API
    const token = await apiLogin(context);
    const uploadResp = await context.request.post('/api/v1/datasets', {
      headers: { Authorization: `Bearer ${token}` },
      multipart: {
        file: {
          name: 'agent_rec_ahu.csv',
          mimeType: 'text/csv',
          buffer: Buffer.from(
            await (await import('fs')).promises.readFile(AHU_CSV, 'utf-8'),
          ),
        },
        name: 'Agent Recommendation AHU',
        equipment_type: 'air_handler',
      },
    });
    const ds = await uploadResp.json();

    // Navigate to agent and ask for recommendation
    await page.goto('/agent');
    await page.waitForSelector('[data-testid="agent-page"]');

    const chatInput = page.locator('[data-testid="chat-input"]');
    await chatInput.fill(
      `I uploaded an AHU dataset (${ds.id}) with supply_temp, return_temp, fan_speed columns. What model architecture do you recommend for anomaly detection?`,
    );
    await page.getByRole('button', { name: /send/i }).click();

    // Wait for agent response
    const agentMessage = page.locator('[data-testid="agent-message"]').last();
    await expect(agentMessage).toBeVisible({ timeout: 60_000 });

    // Response should mention an architecture
    const responseText = await agentMessage.textContent();
    expect(responseText).toMatch(/lstm|gru|autoencoder|sentinel|architecture|model/i);
  });

  /* ── Compare two models ────────────────────────────── */

  test('compare two models and verify comparison view', async ({ page }) => {
    await page.goto('/models');
    await page.waitForSelector('[data-testid="models-page"]');

    const modelCards = page.locator('[data-testid="model-card"]');
    const count = await modelCards.count();

    if (count >= 2) {
      const checkbox1 = modelCards.nth(0).locator('[data-testid="compare-checkbox"]');
      const checkbox2 = modelCards.nth(1).locator('[data-testid="compare-checkbox"]');

      if ((await checkbox1.count()) > 0 && (await checkbox2.count()) > 0) {
        await checkbox1.check();
        await checkbox2.check();

        const compareBtn = page.getByRole('button', { name: /compare/i });
        await expect(compareBtn).toBeVisible();
        await compareBtn.click();

        const comparison = page.locator('[data-testid="model-comparison"]');
        await expect(comparison).toBeVisible();

        // Should show metrics side-by-side
        await expect(comparison.getByText(/precision/i)).toBeVisible();
        await expect(comparison.getByText(/recall/i)).toBeVisible();
        await expect(comparison.getByText(/f1/i)).toBeVisible();

        // Should show both model names
        const comparisonText = await comparison.textContent();
        expect(comparisonText?.length).toBeGreaterThan(20);
      }
    }
  });

  /* ── Download model binary ─────────────────────────── */

  test('download model binary', async ({ page }) => {
    await page.goto('/models');
    await page.waitForSelector('[data-testid="models-page"]');

    const modelCards = page.locator('[data-testid="model-card"]');
    if ((await modelCards.count()) > 0) {
      await modelCards.first().click();
      await page.waitForURL('**/models/**');

      const downloadBtn = page.getByRole('button', { name: /download/i });
      if ((await downloadBtn.count()) > 0) {
        const downloadPromise = page.waitForEvent('download');
        await downloadBtn.click();
        const download = await downloadPromise;

        // Verify the downloaded file
        const filename = download.suggestedFilename();
        expect(filename).toMatch(/\.(axonml|bin|onnx|pt)$/);

        const filePath = await download.path();
        expect(filePath).toBeTruthy();
      }
    }
  });

  /* ── Generate evaluation report ────────────────────── */

  test('generate evaluation report', async ({ page }) => {
    await page.goto('/evaluation');
    await page.waitForSelector('[data-testid="evaluation-page"]');

    const evaluations = page.locator('[data-testid="evaluation-card"]');
    if ((await evaluations.count()) > 0) {
      await evaluations.first().click();

      const metricsPanel = page.locator('[data-testid="metrics-panel"]');
      await expect(metricsPanel).toBeVisible();

      // Look for a report generation button
      const reportBtn = page.getByRole('button', { name: /report|generate|export/i });
      if ((await reportBtn.count()) > 0) {
        const downloadPromise = page.waitForEvent('download').catch(() => null);
        await reportBtn.click();

        // Either a download starts or a report view is shown inline
        const download = await downloadPromise;
        const reportView = page.locator('[data-testid="evaluation-report"]');

        if (download) {
          const filename = download.suggestedFilename();
          expect(filename).toMatch(/\.(pdf|html|json|csv)$/);
        } else if ((await reportView.count()) > 0) {
          await expect(reportView).toBeVisible();
        }
      }

      // Verify metrics are present regardless
      const expectedMetrics = ['accuracy', 'precision', 'recall', 'f1'];
      for (const metric of expectedMetrics) {
        await expect(metricsPanel.getByText(metric, { exact: false })).toBeVisible();
      }
    }
  });

  /* ── View deployment certificate ───────────────────── */

  test('view deployment certificate', async ({ page }) => {
    await page.goto('/deployment');
    await page.waitForSelector('[data-testid="deployment-page"]');

    const deploymentRows = page.locator(
      '[data-testid="deployment-table"] tbody tr, [data-testid="deployment-card"]',
    );
    const count = await deploymentRows.count();

    if (count > 0) {
      // Find a completed deployment
      const completedRow = page
        .locator(
          '[data-testid="deployment-table"] tbody tr:has-text("deployed"), [data-testid="deployment-card"]:has-text("deployed")',
        )
        .first();

      if ((await completedRow.count()) > 0) {
        await completedRow.click();

        const detail = page.locator('[data-testid="deployment-detail"]');
        if ((await detail.count()) > 0) {
          await expect(detail).toBeVisible();

          // Look for certificate information
          const certSection = detail.locator(
            '[data-testid="deployment-certificate"], [data-testid="certificate-info"]',
          );
          if ((await certSection.count()) > 0) {
            await expect(certSection).toBeVisible();

            // Certificate should show signing information
            const certText = await certSection.textContent();
            expect(certText?.length).toBeGreaterThan(0);
          }

          // Verify deployment step details are shown
          const steps = detail.locator('[data-testid="deployment-step"]');
          const stepCount = await steps.count();
          expect(stepCount).toBeGreaterThanOrEqual(1);
        }
      }
    }
  });
});
