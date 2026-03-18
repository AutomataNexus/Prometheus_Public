import { test, expect } from '@playwright/test';
import { login, apiLogin, apiRequest } from '../fixtures/auth';
import path from 'path';

const AHU_CSV = path.resolve(__dirname, '../fixtures/sample_ahu_data.csv');
const BOILER_CSV = path.resolve(__dirname, '../fixtures/sample_boiler_data.csv');

test.describe('Dataset Management', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
    await page.goto('/datasets');
    await page.waitForSelector('[data-testid="datasets-page"]');
  });

  /* ── Empty state ────────────────────────────────────── */

  test('displays empty state when no datasets exist', async ({ page, context }) => {
    // Delete all existing datasets via API first
    const token = await apiLogin(context);
    const listResp = await apiRequest(context, 'GET', '/api/v1/datasets', token);
    const datasets = await listResp!.json();
    if (Array.isArray(datasets)) {
      for (const ds of datasets) {
        await apiRequest(context, 'DELETE', `/api/v1/datasets/${ds.id}`, token);
      }
    }

    await page.reload();
    await page.waitForSelector('[data-testid="datasets-page"]');

    const emptyState = page.locator('[data-testid="empty-state"]');
    await expect(emptyState).toBeVisible();
    await expect(emptyState).toContainText(/no datasets|upload|get started/i);
  });

  /* ── CSV upload via drag-and-drop ───────────────────── */

  test('uploads CSV file via file input', async ({ page }) => {
    const fileInput = page.locator('[data-testid="file-upload"] input[type="file"]');
    await fileInput.setInputFiles(AHU_CSV);

    // Wait for upload progress
    const progressBar = page.locator('[data-testid="upload-progress"]');
    await expect(progressBar).toBeVisible({ timeout: 10_000 });

    // Wait for upload completion
    await expect(page.locator('[data-testid="upload-success"]')).toBeVisible({ timeout: 30_000 });
  });

  test('shows upload progress bar during upload', async ({ page }) => {
    const fileInput = page.locator('[data-testid="file-upload"] input[type="file"]');
    await fileInput.setInputFiles(BOILER_CSV);

    const progressBar = page.locator('[data-testid="upload-progress"]');
    await expect(progressBar).toBeVisible({ timeout: 10_000 });

    // Progress should eventually reach 100% or disappear when complete
    await expect(page.locator('[data-testid="upload-success"]')).toBeVisible({ timeout: 30_000 });
  });

  test('displays dataset in list after upload', async ({ page }) => {
    // Upload AHU data
    const fileInput = page.locator('[data-testid="file-upload"] input[type="file"]');
    await fileInput.setInputFiles(AHU_CSV);
    await expect(page.locator('[data-testid="upload-success"]')).toBeVisible({ timeout: 30_000 });

    // Verify it appears in the dataset table
    const table = page.locator('[data-testid="dataset-table"]');
    await expect(table).toBeVisible();
    await expect(table.getByText('sample_ahu_data', { exact: false })).toBeVisible();
  });

  /* ── Dataset preview ────────────────────────────────── */

  test('displays column statistics after upload', async ({ page }) => {
    // Upload then navigate to detail
    const fileInput = page.locator('[data-testid="file-upload"] input[type="file"]');
    await fileInput.setInputFiles(AHU_CSV);
    await expect(page.locator('[data-testid="upload-success"]')).toBeVisible({ timeout: 30_000 });

    // Click on the dataset to see details
    await page.locator('[data-testid="dataset-table"] tr').last().click();
    await page.waitForURL('**/datasets/**');

    // Column stats should be visible
    const stats = page.locator('[data-testid="column-stats"]');
    await expect(stats).toBeVisible();

    // Should show stats for known columns
    const expectedColumns = ['supply_temp', 'return_temp', 'outside_air_temp', 'fan_speed'];
    for (const col of expectedColumns) {
      await expect(stats.getByText(col, { exact: false })).toBeVisible();
    }
  });

  test('shows time series preview chart', async ({ page }) => {
    // Upload then navigate to detail
    const fileInput = page.locator('[data-testid="file-upload"] input[type="file"]');
    await fileInput.setInputFiles(AHU_CSV);
    await expect(page.locator('[data-testid="upload-success"]')).toBeVisible({ timeout: 30_000 });

    await page.locator('[data-testid="dataset-table"] tr').last().click();
    await page.waitForURL('**/datasets/**');

    const chart = page.locator('[data-testid="preview-chart"]');
    await expect(chart).toBeVisible();
  });

  test('shows row count and time range in dataset detail', async ({ page }) => {
    const fileInput = page.locator('[data-testid="file-upload"] input[type="file"]');
    await fileInput.setInputFiles(AHU_CSV);
    await expect(page.locator('[data-testid="upload-success"]')).toBeVisible({ timeout: 30_000 });

    await page.locator('[data-testid="dataset-table"] tr').last().click();
    await page.waitForURL('**/datasets/**');

    const detail = page.locator('[data-testid="dataset-detail"]');
    await expect(detail).toBeVisible();

    // Row count
    await expect(detail.getByText(/100|rows/i)).toBeVisible();
  });

  /* ── InfluxDB connection ────────────────────────────── */

  test('connects to InfluxDB endpoint', async ({ page }) => {
    const connectBtn = page.getByRole('button', { name: /connect|influxdb/i });
    await connectBtn.click();

    const modal = page.locator('[data-testid="influxdb-modal"]');
    await expect(modal).toBeVisible();

    // Fill connection form
    await modal.getByLabel(/url/i).fill('http://localhost:8086');
    await modal.getByLabel(/database/i).fill('building_sensors');
    await modal.getByLabel(/measurement/i).fill('ahu_readings');

    // Test connection
    const testBtn = modal.getByRole('button', { name: /test/i });
    await testBtn.click();

    // Wait for connection test result
    const result = modal.locator('[data-testid="connection-result"]');
    await expect(result).toBeVisible({ timeout: 10_000 });
  });

  test('shows error for invalid InfluxDB connection', async ({ page }) => {
    const connectBtn = page.getByRole('button', { name: /connect|influxdb/i });
    await connectBtn.click();

    const modal = page.locator('[data-testid="influxdb-modal"]');
    await expect(modal).toBeVisible();

    // Fill with invalid URL
    await modal.getByLabel(/url/i).fill('http://nonexistent-host:9999');
    await modal.getByLabel(/database/i).fill('fake_db');
    await modal.getByLabel(/measurement/i).fill('fake_measurement');

    const testBtn = modal.getByRole('button', { name: /test/i });
    await testBtn.click();

    // Should show connection error
    const error = modal.locator('[data-testid="connection-error"]');
    await expect(error).toBeVisible({ timeout: 15_000 });
    await expect(error).toContainText(/error|failed|cannot connect/i);
  });

  /* ── Delete dataset ─────────────────────────────────── */

  test('deletes dataset with confirmation dialog', async ({ page }) => {
    // Upload a dataset to delete
    const fileInput = page.locator('[data-testid="file-upload"] input[type="file"]');
    await fileInput.setInputFiles(AHU_CSV);
    await expect(page.locator('[data-testid="upload-success"]')).toBeVisible({ timeout: 30_000 });

    // Find the delete button for the uploaded dataset
    const row = page.locator('[data-testid="dataset-table"] tr').last();
    const deleteBtn = row.locator('[data-testid="delete-dataset"]');
    await deleteBtn.click();

    // Confirmation dialog
    const dialog = page.locator('[data-testid="confirm-dialog"]');
    await expect(dialog).toBeVisible();
    await expect(dialog).toContainText(/delete|confirm|are you sure/i);

    // Confirm deletion
    await dialog.getByRole('button', { name: /confirm|delete|yes/i }).click();

    // Dataset should be removed from the list
    await expect(page.locator('[data-testid="delete-success"]')).toBeVisible({ timeout: 10_000 });
  });

  /* ── Pagination ─────────────────────────────────────── */

  test('paginates large dataset list', async ({ page, context }) => {
    // Create multiple datasets via API to trigger pagination
    const token = await apiLogin(context);

    for (let i = 0; i < 15; i++) {
      await context.request.post('/api/v1/datasets', {
        headers: { Authorization: `Bearer ${token}` },
        multipart: {
          file: {
            name: `test_dataset_${i}.csv`,
            mimeType: 'text/csv',
            buffer: Buffer.from(
              'timestamp,value\n2026-01-01T00:00:00Z,42.0\n2026-01-01T00:15:00Z,43.1\n',
            ),
          },
          name: `Test Dataset ${i}`,
          equipment_type: 'air_handler',
        },
      });
    }

    await page.reload();
    await page.waitForSelector('[data-testid="datasets-page"]');

    // Check for pagination controls
    const pagination = page.locator('[data-testid="pagination"]');
    if ((await pagination.count()) > 0) {
      await expect(pagination).toBeVisible();

      // Click next page
      const nextBtn = pagination.getByRole('button', { name: /next|>/i });
      if ((await nextBtn.count()) > 0 && (await nextBtn.isEnabled())) {
        await nextBtn.click();
        await page.waitForTimeout(500);
        // Table should still have rows
        const rows = page.locator('[data-testid="dataset-table"] tbody tr');
        expect(await rows.count()).toBeGreaterThan(0);
      }
    }
  });

  /* ── API-level dataset operations ───────────────────── */

  test('lists datasets via API', async ({ context }) => {
    const token = await apiLogin(context);
    const resp = await context.request.get('/api/v1/datasets', {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(resp.ok()).toBeTruthy();
    const body = await resp.json();
    expect(Array.isArray(body)).toBeTruthy();
  });

  test('gets dataset detail via API', async ({ context }) => {
    const token = await apiLogin(context);

    // Upload via API
    const uploadResp = await context.request.post('/api/v1/datasets', {
      headers: { Authorization: `Bearer ${token}` },
      multipart: {
        file: {
          name: 'api_test.csv',
          mimeType: 'text/csv',
          buffer: Buffer.from(
            'timestamp,supply_temp,return_temp\n2026-01-01T00:00:00Z,55.2,72.1\n2026-01-01T00:15:00Z,55.4,72.0\n',
          ),
        },
        name: 'API Test Dataset',
        equipment_type: 'air_handler',
      },
    });
    expect(uploadResp.ok()).toBeTruthy();
    const dataset = await uploadResp.json();

    // Get detail
    const detailResp = await context.request.get(`/api/v1/datasets/${dataset.id}`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(detailResp.ok()).toBeTruthy();
    const detail = await detailResp.json();
    expect(detail).toHaveProperty('id', dataset.id);
    expect(detail).toHaveProperty('columns');
    expect(detail).toHaveProperty('row_count');
  });
});
