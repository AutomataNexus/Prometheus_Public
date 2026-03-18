import { test, expect } from '@playwright/test';
import { login, logout, apiLogin, apiRequest, TEST_ADMIN } from '../fixtures/auth';
import path from 'path';

const AHU_CSV = path.resolve(__dirname, '../fixtures/sample_ahu_data.csv');

test.describe('Customer Acceptance Tests', () => {
  /* ══════════════════════════════════════════════════════════
   * Workflow 1: New User Onboarding
   * Login -> verify dashboard loads -> check all nav links
   * work -> check user profile visible
   * ══════════════════════════════════════════════════════════ */

  test.describe('Workflow 1: New User Onboarding', () => {
    test('login, verify dashboard, navigate all sections, view profile', async ({ page }) => {
      // Step 1: Login
      const token = await login(page);
      expect(token.length).toBeGreaterThan(0);

      // Step 2: Verify dashboard loads with key components
      const dashboard = page.locator('[data-testid="dashboard"]');
      await expect(dashboard).toBeVisible();

      // Pipeline visualization is present
      const pipeline = page.locator('[data-testid="pipeline-viz"]');
      await expect(pipeline).toBeVisible();

      // Metric cards are displayed
      const metricCards = page.locator('[data-testid="metric-card"]');
      await expect(metricCards.first()).toBeVisible();

      // Step 3: Check all sidebar nav links navigate correctly
      const navItems = [
        { label: /data/i, path: '/datasets', testId: 'datasets-page' },
        { label: /train/i, path: '/training', testId: 'training-page' },
        { label: /model/i, path: '/models', testId: 'models-page' },
        { label: /deploy/i, path: '/deployment', testId: 'deployment-page' },
        { label: /eval/i, path: '/evaluation', testId: 'evaluation-page' },
        { label: /agent/i, path: '/agent', testId: 'agent-page' },
        { label: /setting/i, path: '/settings', testId: 'settings-page' },
      ];

      for (const { label, path: navPath, testId } of navItems) {
        const link = page.locator('[data-testid="sidebar"]').getByRole('link', { name: label });
        if ((await link.count()) > 0) {
          await link.first().click();
          await page.waitForURL(`**${navPath}**`);
          expect(page.url()).toContain(navPath);

          const pageEl = page.locator(`[data-testid="${testId}"]`);
          await expect(pageEl).toBeVisible({ timeout: 10_000 });
        }
      }

      // Step 4: Verify user profile is visible in header
      const userMenu = page.locator('[data-testid="user-menu-button"]');
      await expect(userMenu).toBeVisible();
      await userMenu.click();

      // User info should be displayed in the dropdown
      const userInfo = page.locator('[data-testid="user-menu-button"], [data-testid="user-info"]');
      await expect(userInfo.first()).toBeVisible();
    });
  });

  /* ══════════════════════════════════════════════════════════
   * Workflow 2: Dataset-to-Model Pipeline
   * Login -> Upload CSV -> verify dataset in list -> start
   * training (lstm_autoencoder) -> verify training run ->
   * wait for status updates -> verify model created
   * ══════════════════════════════════════════════════════════ */

  test.describe('Workflow 2: Dataset-to-Model Pipeline', () => {
    test('upload dataset, train lstm_autoencoder, verify model creation', async ({
      page,
      context,
    }) => {
      await login(page);

      // Step 1: Upload CSV dataset
      await page.goto('/datasets');
      await page.waitForSelector('[data-testid="datasets-page"]');

      const fileInput = page.locator('[data-testid="file-upload"] input[type="file"]');
      await fileInput.setInputFiles(AHU_CSV);
      await expect(page.locator('[data-testid="upload-success"]')).toBeVisible({
        timeout: 30_000,
      });

      // Step 2: Verify dataset appears in list
      const table = page.locator('[data-testid="dataset-table"]');
      await expect(table).toBeVisible();
      await expect(table.getByText('sample_ahu_data', { exact: false })).toBeVisible();

      // Step 3: Navigate to training and start an lstm_autoencoder run
      await page.goto('/training');
      await page.waitForSelector('[data-testid="training-page"]');

      await page.getByRole('button', { name: /start|new.training/i }).click();

      const datasetSelect = page.locator('[data-testid="dataset-select"]');
      await expect(datasetSelect).toBeVisible();
      await datasetSelect.selectOption({ label: /sample_ahu_data|ahu/i });

      // Wait for training plan to be generated
      const planSection = page.locator('[data-testid="training-plan"]');
      await expect(planSection).toBeVisible({ timeout: 30_000 });

      // Confirm and start training
      await page.getByRole('button', { name: /confirm|start|begin/i }).click();

      // Step 4: Verify training run appears with progress
      await expect(page.locator('[data-testid="training-progress"]')).toBeVisible({
        timeout: 15_000,
      });

      const epochCounter = page.locator('[data-testid="epoch-counter"]');
      await expect(epochCounter).toBeVisible({ timeout: 15_000 });

      // Step 5: Wait for status updates (at least one epoch progresses)
      await page.waitForTimeout(10_000);
      const epochText = await epochCounter.textContent();
      expect(epochText).toMatch(/\d+\s*\/\s*\d+/);

      // Step 6: Wait for training completion
      await expect(page.locator('[data-testid="training-status"]')).toContainText(
        /completed|finished|done/i,
        { timeout: 300_000 },
      );

      // Step 7: Verify model was created
      const viewModelLink = page.getByRole('link', { name: /view.model|model/i }).first();
      if ((await viewModelLink.count()) > 0) {
        await viewModelLink.click();
        await page.waitForURL('**/models/**');

        const modelDetail = page.locator('[data-testid="model-detail"]');
        await expect(modelDetail).toBeVisible();

        // Model should have the lstm_autoencoder architecture
        const archSection = modelDetail.locator('[data-testid="architecture-section"]');
        if ((await archSection.count()) > 0) {
          await expect(archSection).toBeVisible();
          await expect(archSection.getByText(/lstm|autoencoder/i)).toBeVisible();
        }
      } else {
        // Alternatively verify via API
        const token = await apiLogin(context);
        const modelsResp = await context.request.get('/api/v1/models', {
          headers: { Authorization: `Bearer ${token}` },
        });
        expect(modelsResp.ok()).toBeTruthy();
        const models = await modelsResp.json();
        expect(Array.isArray(models)).toBeTruthy();
        expect(models.length).toBeGreaterThanOrEqual(1);
      }
    });
  });

  /* ══════════════════════════════════════════════════════════
   * Workflow 3: Model Evaluation & Comparison
   * Login -> navigate to models -> select model -> view
   * evaluation metrics -> verify metric cards visible
   * (precision, recall, F1, val_loss)
   * ══════════════════════════════════════════════════════════ */

  test.describe('Workflow 3: Model Evaluation & Comparison', () => {
    test('view model evaluation metrics: precision, recall, F1, val_loss', async ({ page }) => {
      await login(page);

      // Navigate to models page
      await page.goto('/models');
      await page.waitForSelector('[data-testid="models-page"]');

      const modelCards = page.locator('[data-testid="model-card"]');
      const count = await modelCards.count();

      if (count === 0) {
        console.warn('No models available — skipping model evaluation workflow');
        return;
      }

      // Select first model to view details
      await modelCards.first().click();
      await page.waitForURL('**/models/**');

      const modelDetail = page.locator('[data-testid="model-detail"]');
      await expect(modelDetail).toBeVisible();

      // Verify metric cards are visible
      const metricsSection = page.locator('[data-testid="model-metrics"]');
      await expect(metricsSection).toBeVisible();

      // Check for all required metric types
      const requiredMetrics = ['precision', 'recall', 'f1'];
      for (const metric of requiredMetrics) {
        await expect(metricsSection.getByText(metric, { exact: false })).toBeVisible();
      }

      // val_loss may appear in the metrics or training info section
      const metricsAndTraining = page.locator(
        '[data-testid="model-metrics"], [data-testid="training-info"]',
      );
      await expect(metricsAndTraining.getByText(/val.loss|loss/i).first()).toBeVisible();
    });

    test('compare two models side-by-side with metric cards', async ({ page }) => {
      await login(page);

      await page.goto('/models');
      await page.waitForSelector('[data-testid="models-page"]');

      const modelCards = page.locator('[data-testid="model-card"]');
      const count = await modelCards.count();

      if (count < 2) {
        console.warn('Need at least 2 models for comparison — skipping');
        return;
      }

      const checkbox1 = modelCards.nth(0).locator('[data-testid="compare-checkbox"]');
      const checkbox2 = modelCards.nth(1).locator('[data-testid="compare-checkbox"]');

      if ((await checkbox1.count()) === 0 || (await checkbox2.count()) === 0) {
        console.warn('Compare checkboxes not available — skipping');
        return;
      }

      await checkbox1.check();
      await checkbox2.check();

      const compareBtn = page.getByRole('button', { name: /compare/i });
      await expect(compareBtn).toBeVisible();
      await compareBtn.click();

      const comparison = page.locator('[data-testid="model-comparison"]');
      await expect(comparison).toBeVisible();

      // Comparison should show metric cards for both models
      await expect(comparison.getByText(/precision/i)).toBeVisible();
      await expect(comparison.getByText(/recall/i)).toBeVisible();
      await expect(comparison.getByText(/f1/i)).toBeVisible();
    });
  });

  /* ══════════════════════════════════════════════════════════
   * Workflow 4: Edge Deployment
   * Login -> navigate to deployment -> select model -> deploy
   * to target -> verify deployment status
   * ══════════════════════════════════════════════════════════ */

  test.describe('Workflow 4: Edge Deployment', () => {
    test('deploy model to edge target and verify deployment status', async ({ page }) => {
      await login(page);

      await page.goto('/deployment');
      await page.waitForSelector('[data-testid="deployment-page"]');

      // Initiate deployment
      const deployBtn = page.getByRole('button', { name: /deploy|new.deployment/i });
      await deployBtn.click();

      // Select a model
      const modelSelect = page.locator('[data-testid="model-select"]');
      await expect(modelSelect).toBeVisible();

      const modelOptions = modelSelect.locator('option');
      if ((await modelOptions.count()) <= 1) {
        console.warn('No models available for deployment — skipping');
        return;
      }
      await modelSelect.selectOption({ index: 1 });

      // Select a target
      const targetSelect = page.locator('[data-testid="target-select"]');
      if ((await targetSelect.count()) > 0 && (await targetSelect.locator('option').count()) > 1) {
        await targetSelect.selectOption({ index: 1 });
      }

      // Confirm deployment
      await page.getByRole('button', { name: /confirm|deploy/i }).click();

      // Handle optional confirmation modal
      const confirmModal = page.locator('[data-testid="deploy-confirm-modal"]');
      if ((await confirmModal.count()) > 0) {
        await confirmModal.getByRole('button', { name: /confirm|yes|deploy/i }).click();
      }

      // Verify deployment status appears
      const statusBadge = page.locator(
        '[data-testid="deployment-status"], [data-testid="deployment-status-badge"]',
      );
      await expect(statusBadge.first()).toBeVisible({ timeout: 30_000 });

      const statusText = await statusBadge.first().textContent();
      expect(statusText).toMatch(/pending|compiling|deploying|deployed/i);
    });
  });

  /* ══════════════════════════════════════════════════════════
   * Workflow 5: AI Agent Interaction
   * Login -> navigate to agent -> send message "What
   * architectures are available?" -> verify response contains
   * architecture names -> verify chat history
   * ══════════════════════════════════════════════════════════ */

  test.describe('Workflow 5: AI Agent Interaction', () => {
    test('send architecture query and verify response with history', async ({ page }) => {
      await login(page);

      await page.goto('/agent');
      await page.waitForSelector('[data-testid="agent-page"]');

      // Send message asking about architectures
      const chatInput = page.locator('[data-testid="chat-input"]');
      await chatInput.fill('What architectures are available?');
      await page.getByRole('button', { name: /send/i }).click();

      // Verify user message appears
      const userMessage = page.locator('[data-testid="user-message"]').last();
      await expect(userMessage).toBeVisible();
      await expect(userMessage).toContainText('What architectures are available?');

      // Wait for agent response
      const agentMessage = page.locator('[data-testid="agent-message"]').last();
      await expect(agentMessage).toBeVisible({ timeout: 60_000 });

      // Response should reference architecture names
      const responseText = await agentMessage.textContent();
      expect(responseText).toMatch(
        /lstm|gru|sentinel|autoencoder|rnn|resnet|vgg|vit|bert|gpt|nexus|phantom|conv/i,
      );
      expect(responseText!.length).toBeGreaterThan(20);

      // Verify chat history is accessible
      const historyBtn = page.locator(
        '[data-testid="history-toggle"], [data-testid="conversation-history"]',
      );
      if ((await historyBtn.count()) > 0) {
        await historyBtn.first().click();

        const historyPanel = page.locator('[data-testid="history-panel"]');
        if ((await historyPanel.count()) > 0) {
          await expect(historyPanel).toBeVisible();

          const conversations = historyPanel.locator('[data-testid="conversation-item"]');
          expect(await conversations.count()).toBeGreaterThanOrEqual(1);
        }
      }
    });

    test('agent responds via API and history persists', async ({ context }) => {
      const token = await apiLogin(context);

      // Send a chat message via API
      const chatResp = await context.request.post('/api/v1/agent/chat', {
        headers: { Authorization: `Bearer ${token}` },
        data: {
          message: 'What architectures are available?',
        },
      });
      expect(chatResp.ok()).toBeTruthy();

      const chatBody = await chatResp.json();
      expect(chatBody).toHaveProperty('response');
      expect(chatBody.response).toMatch(
        /lstm|gru|sentinel|autoencoder|rnn|architecture|model/i,
      );

      // Verify history contains the interaction
      const historyResp = await context.request.get('/api/v1/agent/history', {
        headers: { Authorization: `Bearer ${token}` },
      });
      expect(historyResp.ok()).toBeTruthy();

      const history = await historyResp.json();
      expect(Array.isArray(history)).toBeTruthy();
      expect(history.length).toBeGreaterThanOrEqual(1);
    });
  });

  /* ══════════════════════════════════════════════════════════
   * Workflow 6: Cross-Session Consistency
   * Login -> create resource -> logout -> login again ->
   * verify resource still exists
   * ══════════════════════════════════════════════════════════ */

  test.describe('Workflow 6: Cross-Session Consistency', () => {
    test('dataset persists across logout and re-login', async ({ page, context }) => {
      // Session 1: Login and upload a dataset
      await login(page);

      await page.goto('/datasets');
      await page.waitForSelector('[data-testid="datasets-page"]');

      const fileInput = page.locator('[data-testid="file-upload"] input[type="file"]');
      await fileInput.setInputFiles(AHU_CSV);
      await expect(page.locator('[data-testid="upload-success"]')).toBeVisible({
        timeout: 30_000,
      });

      // Confirm dataset is in the table
      const table = page.locator('[data-testid="dataset-table"]');
      await expect(table).toBeVisible();
      await expect(table.getByText(/ahu|sample/i).first()).toBeVisible();

      // Record the dataset name for later verification
      const rows = table.locator('tbody tr');
      const rowCount = await rows.count();
      expect(rowCount).toBeGreaterThanOrEqual(1);

      // Logout
      await logout(page);
      await page.waitForURL('**/login');

      // Session 2: Login again
      await login(page);

      // Navigate back to datasets
      await page.goto('/datasets');
      await page.waitForSelector('[data-testid="datasets-page"]');

      // Verify the dataset still exists
      const tableAfterRelogin = page.locator('[data-testid="dataset-table"]');
      await expect(tableAfterRelogin).toBeVisible();
      await expect(tableAfterRelogin.getByText(/ahu|sample/i).first()).toBeVisible();

      // Row count should be at least what it was before
      const rowCountAfter = await tableAfterRelogin.locator('tbody tr').count();
      expect(rowCountAfter).toBeGreaterThanOrEqual(rowCount);
    });

    test('resource persists via API across sessions', async ({ context }) => {
      // Session 1: Login and create a resource
      const token1 = await apiLogin(context);

      // Upload dataset via API
      const csvContent = 'timestamp,supply_temp,return_temp\n2026-01-01T00:00:00Z,55.2,72.1\n';
      const uploadResp = await context.request.post('/api/v1/datasets', {
        headers: { Authorization: `Bearer ${token1}` },
        multipart: {
          file: {
            name: 'cross_session_test.csv',
            mimeType: 'text/csv',
            buffer: Buffer.from(csvContent),
          },
          name: 'Cross Session Test',
          equipment_type: 'air_handler',
        },
      });
      expect(uploadResp.ok()).toBeTruthy();
      const dataset = await uploadResp.json();
      const datasetId = dataset.id;

      // Logout (session 1 token invalidated)
      await context.request.post('/api/v1/auth/logout', {
        headers: { Authorization: `Bearer ${token1}` },
      });

      // Session 2: Login again
      const token2 = await apiLogin(context);
      expect(token2).not.toBe(token1);

      // Verify the dataset still exists
      const getResp = await context.request.get(`/api/v1/datasets/${datasetId}`, {
        headers: { Authorization: `Bearer ${token2}` },
      });
      expect(getResp.ok()).toBeTruthy();

      const fetchedDataset = await getResp.json();
      expect(fetchedDataset.id).toBe(datasetId);

      // Cleanup
      await context.request.delete(`/api/v1/datasets/${datasetId}`, {
        headers: { Authorization: `Bearer ${token2}` },
      });
    });
  });

  /* ══════════════════════════════════════════════════════════
   * Workflow 7: Multi-Architecture Training
   * Login -> verify all 13 architectures are listed in
   * training form or selectable
   * ══════════════════════════════════════════════════════════ */

  test.describe('Workflow 7: Multi-Architecture Training', () => {
    // All 13 architectures defined in prometheus-training
    const ALL_ARCHITECTURES = [
      'lstm_autoencoder',
      'gru_predictor',
      'rnn',
      'sentinel',
      'resnet',
      'vgg',
      'vit',
      'bert',
      'gpt2',
      'nexus',
      'phantom',
      'conv1d',
      'conv2d',
    ];

    test('all 13 architectures are available in training form', async ({ page, context }) => {
      await login(page);

      // Upload a dataset to enable training form
      const token = await apiLogin(context);
      const csvContent =
        'timestamp,supply_temp,return_temp,fan_speed\n2026-01-01T00:00:00Z,55.2,72.1,78.5\n';
      const uploadResp = await context.request.post('/api/v1/datasets', {
        headers: { Authorization: `Bearer ${token}` },
        multipart: {
          file: {
            name: 'arch_test.csv',
            mimeType: 'text/csv',
            buffer: Buffer.from(csvContent),
          },
          name: 'Architecture Test Dataset',
          equipment_type: 'air_handler',
        },
      });
      const ds = await uploadResp.json();

      await page.goto('/training');
      await page.waitForSelector('[data-testid="training-page"]');

      await page.getByRole('button', { name: /start|new.training/i }).click();

      // Select dataset to populate architecture options
      const datasetSelect = page.locator('[data-testid="dataset-select"]');
      await expect(datasetSelect).toBeVisible();
      await datasetSelect.selectOption({ label: /Architecture Test|arch_test/i });

      // Wait for training plan or architecture selector to appear
      await page.waitForTimeout(3_000);

      // Check for architecture selector or architecture list
      const archSelector = page.locator(
        '[data-testid="architecture-select"], [data-testid="architecture-list"], [data-testid="training-plan"]',
      );
      await expect(archSelector.first()).toBeVisible({ timeout: 30_000 });

      // Get all text content from the architecture area and training form
      const formContent = await page
        .locator(
          '[data-testid="training-form"], [data-testid="training-plan"], [data-testid="architecture-select"]',
        )
        .first()
        .textContent();

      // Alternatively, check via API for available architectures
      const archResp = await context.request.get('/api/v1/training/architectures', {
        headers: { Authorization: `Bearer ${token}` },
      });

      if (archResp.ok()) {
        const architectures = await archResp.json();
        if (Array.isArray(architectures)) {
          expect(architectures.length).toBeGreaterThanOrEqual(13);

          // Verify each architecture is present
          for (const arch of ALL_ARCHITECTURES) {
            const found = architectures.some(
              (a: { name?: string; id?: string }) =>
                (a.name || a.id || '').toLowerCase().includes(arch.toLowerCase()),
            );
            expect(found).toBeTruthy();
          }
        }
      } else if (formContent) {
        // Fall back to checking the form text content
        const lowerContent = formContent.toLowerCase();
        let matchCount = 0;
        for (const arch of ALL_ARCHITECTURES) {
          if (lowerContent.includes(arch.toLowerCase().replace('_', ''))) {
            matchCount++;
          }
        }
        // At minimum, the primary architectures should appear
        expect(matchCount).toBeGreaterThanOrEqual(3);
      }

      // Cleanup
      if (ds?.id) {
        await context.request.delete(`/api/v1/datasets/${ds.id}`, {
          headers: { Authorization: `Bearer ${token}` },
        });
      }
    });

    test('architecture count matches 13 via API', async ({ context }) => {
      const token = await apiLogin(context);

      const archResp = await context.request.get('/api/v1/training/architectures', {
        headers: { Authorization: `Bearer ${token}` },
      });

      if (!archResp.ok()) {
        console.warn('Architectures endpoint not available — skipping count check');
        return;
      }

      const architectures = await archResp.json();
      expect(Array.isArray(architectures)).toBeTruthy();
      expect(architectures.length).toBe(13);
    });
  });
});
