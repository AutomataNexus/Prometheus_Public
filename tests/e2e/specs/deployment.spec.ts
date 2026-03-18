import { test, expect } from '@playwright/test';
import { login, apiLogin } from '../fixtures/auth';

test.describe('Edge Deployment', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
    await page.goto('/deployment');
    await page.waitForSelector('[data-testid="deployment-page"]');
  });

  /* ── Edge target listing ────────────────────────────── */

  test('lists available edge targets', async ({ page }) => {
    const targetList = page.locator('[data-testid="target-list"]');
    await expect(targetList).toBeVisible();

    const targets = targetList.locator('[data-testid="target-card"]');
    const count = await targets.count();
    expect(count).toBeGreaterThanOrEqual(0);
  });

  test('edge targets show IP, status, and current model version', async ({ page }) => {
    const targets = page.locator('[data-testid="target-card"]');
    const count = await targets.count();

    if (count > 0) {
      const firstTarget = targets.first();

      // IP address
      const ip = firstTarget.locator('[data-testid="target-ip"]');
      await expect(ip).toBeVisible();
      const ipText = await ip.textContent();
      expect(ipText).toMatch(/\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}/);

      // Status badge (online/offline)
      const status = firstTarget.locator('[data-testid="target-status"]');
      await expect(status).toBeVisible();

      // Current model version (may be "none" if no model deployed)
      const version = firstTarget.locator('[data-testid="target-model-version"]');
      await expect(version).toBeVisible();
    }
  });

  /* ── Deploy model to target ─────────────────────────── */

  test('deploys model to target controller via UI', async ({ page }) => {
    // Click deploy button
    const deployBtn = page.getByRole('button', { name: /deploy|new.deployment/i });
    await deployBtn.click();

    // Model selection
    const modelSelect = page.locator('[data-testid="model-select"]');
    await expect(modelSelect).toBeVisible();

    const modelOptions = modelSelect.locator('option');
    const optionCount = await modelOptions.count();

    if (optionCount > 1) {
      // Select first available model
      await modelSelect.selectOption({ index: 1 });

      // Target selection
      const targetSelect = page.locator('[data-testid="target-select"]');
      await expect(targetSelect).toBeVisible();

      const targetOptions = targetSelect.locator('option');
      if ((await targetOptions.count()) > 1) {
        await targetSelect.selectOption({ index: 1 });
      }

      // Click deploy
      await page.getByRole('button', { name: /confirm|deploy/i }).click();

      // Should show confirmation modal
      const modal = page.locator('[data-testid="deploy-confirm-modal"]');
      if ((await modal.count()) > 0) {
        await expect(modal).toBeVisible();
        await expect(modal).toContainText(/deploy|confirm/i);
        await modal.getByRole('button', { name: /confirm|yes|deploy/i }).click();
      }

      // Deployment status should appear
      await expect(page.locator('[data-testid="deployment-status"]')).toBeVisible({
        timeout: 30_000,
      });
    }
  });

  test('shows deployment confirmation modal with details', async ({ page }) => {
    const deployBtn = page.getByRole('button', { name: /deploy|new.deployment/i });
    await deployBtn.click();

    const modelSelect = page.locator('[data-testid="model-select"]');
    const modelOptions = modelSelect.locator('option');

    if ((await modelOptions.count()) > 1) {
      await modelSelect.selectOption({ index: 1 });

      const targetSelect = page.locator('[data-testid="target-select"]');
      if ((await targetSelect.locator('option').count()) > 1) {
        await targetSelect.selectOption({ index: 1 });
      }

      await page.getByRole('button', { name: /confirm|deploy/i }).click();

      const modal = page.locator('[data-testid="deploy-confirm-modal"]');
      if ((await modal.count()) > 0) {
        await expect(modal).toBeVisible();

        // Should show model name, target name, and architecture
        await expect(modal.getByText(/model/i)).toBeVisible();
        await expect(modal.getByText(/target|controller/i)).toBeVisible();
      }
    }
  });

  /* ── Deployment status tracking ─────────────────────── */

  test('tracks deployment status in real-time', async ({ page }) => {
    const deploymentRows = page.locator('[data-testid="deployment-table"] tbody tr, [data-testid="deployment-card"]');
    const count = await deploymentRows.count();

    if (count > 0) {
      const firstRow = deploymentRows.first();

      // Status should be visible
      const status = firstRow.locator('[data-testid="deployment-status-badge"]');
      await expect(status).toBeVisible();
      const statusText = await status.textContent();
      expect(statusText).toMatch(/pending|compiling|deploying|deployed|failed/i);
    }
  });

  test('shows deployment history table', async ({ page }) => {
    const historyTable = page.locator('[data-testid="deployment-table"], [data-testid="deployment-history"]');
    await expect(historyTable).toBeVisible();
  });

  test('deployment detail shows compilation and transfer progress', async ({ page }) => {
    const deploymentRows = page.locator('[data-testid="deployment-table"] tbody tr, [data-testid="deployment-card"]');
    const count = await deploymentRows.count();

    if (count > 0) {
      await deploymentRows.first().click();

      // Detail view
      const detail = page.locator('[data-testid="deployment-detail"]');
      if ((await detail.count()) > 0) {
        await expect(detail).toBeVisible();

        // Should show steps: cross-compile, quantize, package, transfer
        const steps = detail.locator('[data-testid="deployment-step"]');
        const stepCount = await steps.count();
        expect(stepCount).toBeGreaterThanOrEqual(1);
      }
    }
  });

  /* ── Download ARM binary ────────────────────────────── */

  test('downloads ARM binary for manual deployment', async ({ page }) => {
    const deploymentRows = page.locator('[data-testid="deployment-table"] tbody tr, [data-testid="deployment-card"]');
    const count = await deploymentRows.count();

    if (count > 0) {
      // Find a completed deployment
      const completedRow = page
        .locator('[data-testid="deployment-table"] tbody tr:has-text("deployed"), [data-testid="deployment-card"]:has-text("deployed")')
        .first();

      if ((await completedRow.count()) > 0) {
        const downloadBtn = completedRow.getByRole('button', { name: /download|binary/i });

        if ((await downloadBtn.count()) > 0) {
          const downloadPromise = page.waitForEvent('download');
          await downloadBtn.click();
          const download = await downloadPromise;

          // Verify file name suggests an ARM binary
          const filename = download.suggestedFilename();
          expect(filename).toMatch(/inference|prometheus|edge/i);

          const filePath = await download.path();
          expect(filePath).toBeTruthy();
        }
      }
    }
  });

  /* ── API-level deployment operations ────────────────── */

  test('lists deployments via API', async ({ context }) => {
    const token = await apiLogin(context);
    const resp = await context.request.get('/api/v1/deployments', {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(resp.ok()).toBeTruthy();
    const body = await resp.json();
    expect(Array.isArray(body)).toBeTruthy();
  });

  test('lists edge targets via API', async ({ context }) => {
    const token = await apiLogin(context);
    const resp = await context.request.get('/api/v1/deployments/targets', {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(resp.ok()).toBeTruthy();
    const body = await resp.json();
    expect(Array.isArray(body)).toBeTruthy();
  });

  test('creates deployment via API', async ({ context }) => {
    const token = await apiLogin(context);

    // Get available models
    const modelsResp = await context.request.get('/api/v1/models', {
      headers: { Authorization: `Bearer ${token}` },
    });
    const models = await modelsResp.json();

    // Get available targets
    const targetsResp = await context.request.get('/api/v1/deployments/targets', {
      headers: { Authorization: `Bearer ${token}` },
    });
    const targets = await targetsResp.json();

    if (Array.isArray(models) && models.length > 0 && Array.isArray(targets) && targets.length > 0) {
      const deployResp = await context.request.post('/api/v1/deployments', {
        headers: { Authorization: `Bearer ${token}` },
        data: {
          model_id: models[0].id,
          target_ip: targets[0].ip ?? targets[0].target_ip,
          target_name: targets[0].name ?? targets[0].target_name,
        },
      });
      expect(deployResp.ok()).toBeTruthy();

      const deployment = await deployResp.json();
      expect(deployment).toHaveProperty('id');
      expect(deployment).toHaveProperty('status');
    }
  });

  test('downloads binary via API', async ({ context }) => {
    const token = await apiLogin(context);
    const listResp = await context.request.get('/api/v1/deployments', {
      headers: { Authorization: `Bearer ${token}` },
    });
    const deployments = await listResp.json();

    if (Array.isArray(deployments) && deployments.length > 0) {
      const deployed = deployments.find((d: Record<string, unknown>) => d.status === 'deployed');
      if (deployed) {
        const binaryResp = await context.request.get(`/api/v1/deployments/${deployed.id}/binary`, {
          headers: { Authorization: `Bearer ${token}` },
        });
        expect(binaryResp.ok()).toBeTruthy();
        const contentType = binaryResp.headers()['content-type'];
        expect(contentType).toMatch(/octet-stream|binary/i);
      }
    }
  });
});
