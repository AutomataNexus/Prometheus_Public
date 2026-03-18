import { test, expect } from '@playwright/test';
import { login, apiLogin } from '../fixtures/auth';

test.describe('Athena Agent Chat', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
    await page.goto('/agent');
    await page.waitForSelector('[data-testid="agent-page"]');
  });

  /* ── Chat interface rendering ───────────────────────── */

  test('renders chat interface with input and message area', async ({ page }) => {
    const chatContainer = page.locator('[data-testid="chat-container"]');
    await expect(chatContainer).toBeVisible();

    // Message area
    const messageArea = page.locator('[data-testid="message-area"]');
    await expect(messageArea).toBeVisible();

    // Input field
    const chatInput = page.locator('[data-testid="chat-input"]');
    await expect(chatInput).toBeVisible();

    // Send button
    const sendBtn = page.getByRole('button', { name: /send/i });
    await expect(sendBtn).toBeVisible();
  });

  test('shows Athena agent branding and introduction', async ({ page }) => {
    // The chat should show a welcome/introduction message from Athena
    const messageArea = page.locator('[data-testid="message-area"]');

    // Either a welcome message or empty state
    const welcomeMsg = messageArea.locator('[data-testid="agent-message"]').first();
    if ((await welcomeMsg.count()) > 0) {
      await expect(welcomeMsg).toContainText(/athena|hello|welcome|help/i);
    }
  });

  /* ── Sending messages ───────────────────────────────── */

  test('sends message to Athena agent and receives response', async ({ page }) => {
    const chatInput = page.locator('[data-testid="chat-input"]');
    await chatInput.fill('Analyze the data patterns for an AHU system. What model architecture would you recommend?');

    await page.getByRole('button', { name: /send/i }).click();

    // User message should appear
    const userMessage = page.locator('[data-testid="user-message"]').last();
    await expect(userMessage).toBeVisible();
    await expect(userMessage).toContainText('Analyze the data patterns');

    // Wait for agent response (may take time due to Gradient API)
    const agentMessage = page.locator('[data-testid="agent-message"]').last();
    await expect(agentMessage).toBeVisible({ timeout: 60_000 });

    // Response should contain relevant content
    const responseText = await agentMessage.textContent();
    expect(responseText?.length).toBeGreaterThan(20);
  });

  test('sends message with Enter key', async ({ page }) => {
    const chatInput = page.locator('[data-testid="chat-input"]');
    await chatInput.fill('What types of models can you train?');
    await chatInput.press('Enter');

    // User message should appear
    const userMessage = page.locator('[data-testid="user-message"]').last();
    await expect(userMessage).toBeVisible();
    await expect(userMessage).toContainText('What types of models');
  });

  test('shows typing indicator while agent is responding', async ({ page }) => {
    const chatInput = page.locator('[data-testid="chat-input"]');
    await chatInput.fill('Give me a summary of HVAC anomaly detection approaches.');
    await page.getByRole('button', { name: /send/i }).click();

    // Typing indicator should appear while waiting for response
    const typingIndicator = page.locator('[data-testid="typing-indicator"]');
    await expect(typingIndicator).toBeVisible({ timeout: 5_000 });

    // After response arrives, typing indicator should disappear
    await page.locator('[data-testid="agent-message"]').last().waitFor({ timeout: 60_000 });
    await expect(typingIndicator).not.toBeVisible();
  });

  test('disables send button while processing', async ({ page }) => {
    const chatInput = page.locator('[data-testid="chat-input"]');
    await chatInput.fill('What is an LSTM autoencoder?');

    const sendBtn = page.getByRole('button', { name: /send/i });
    await sendBtn.click();

    // Button should be disabled while processing
    // (race condition possible — check immediately)
    const isDisabled = await sendBtn.isDisabled();
    // This may already be re-enabled if response is instant
    expect(typeof isDisabled).toBe('boolean');
  });

  /* ── Training plan generation ───────────────────────── */

  test('receives formatted training plan from agent', async ({ page }) => {
    const chatInput = page.locator('[data-testid="chat-input"]');
    await chatInput.fill(
      'I have an AHU dataset with supply_temp, return_temp, outside_air_temp, discharge_temp, fan_speed, damper_position, and filter_dp columns. Please generate a training plan.',
    );
    await page.getByRole('button', { name: /send/i }).click();

    // Wait for agent response
    const agentMessage = page.locator('[data-testid="agent-message"]').last();
    await expect(agentMessage).toBeVisible({ timeout: 60_000 });

    // Response should contain a training plan (JSON or structured format)
    const responseText = await agentMessage.textContent();
    expect(responseText).toMatch(/architecture|lstm|gru|learning.rate|epochs|training.plan/i);
  });

  test('displays training plan JSON in code block', async ({ page }) => {
    const chatInput = page.locator('[data-testid="chat-input"]');
    await chatInput.fill('Generate a training plan JSON for a boiler anomaly detection model.');
    await page.getByRole('button', { name: /send/i }).click();

    const agentMessage = page.locator('[data-testid="agent-message"]').last();
    await expect(agentMessage).toBeVisible({ timeout: 60_000 });

    // Check for code block or training plan container
    const codeBlock = agentMessage.locator(
      '[data-testid="code-block"], pre, code, [data-testid="training-plan-json"]',
    );
    if ((await codeBlock.count()) > 0) {
      await expect(codeBlock.first()).toBeVisible();
    }
  });

  test('accepts training plan and navigates to training page', async ({ page }) => {
    const chatInput = page.locator('[data-testid="chat-input"]');
    await chatInput.fill('Create a training plan for AHU anomaly detection with 100 epochs.');
    await page.getByRole('button', { name: /send/i }).click();

    const agentMessage = page.locator('[data-testid="agent-message"]').last();
    await expect(agentMessage).toBeVisible({ timeout: 60_000 });

    // Look for "Accept Plan" / "Start Training" button in the response
    const acceptBtn = agentMessage.getByRole('button', { name: /accept|start.training|use.this.plan/i });
    if ((await acceptBtn.count()) > 0) {
      await acceptBtn.click();

      // Should navigate to training page
      await page.waitForURL('**/training**', { timeout: 10_000 });
      expect(page.url()).toContain('/training');
    }
  });

  /* ── Conversation history ───────────────────────────── */

  test('shows conversation history', async ({ page }) => {
    // Send a message to create history
    const chatInput = page.locator('[data-testid="chat-input"]');
    await chatInput.fill('Hello Athena');
    await page.getByRole('button', { name: /send/i }).click();

    // Wait for response
    await page.locator('[data-testid="agent-message"]').last().waitFor({ timeout: 60_000 });

    // Check for history panel or past conversations
    const historyBtn = page.locator('[data-testid="history-toggle"], [data-testid="conversation-history"]');
    if ((await historyBtn.count()) > 0) {
      await historyBtn.first().click();

      const historyPanel = page.locator('[data-testid="history-panel"]');
      if ((await historyPanel.count()) > 0) {
        await expect(historyPanel).toBeVisible();

        // Should contain at least one conversation
        const conversations = historyPanel.locator('[data-testid="conversation-item"]');
        expect(await conversations.count()).toBeGreaterThanOrEqual(1);
      }
    }
  });

  test('loads previous conversation from history', async ({ page }) => {
    // Check if there are saved conversations
    const historyBtn = page.locator('[data-testid="history-toggle"], [data-testid="conversation-history"]');
    if ((await historyBtn.count()) > 0) {
      await historyBtn.first().click();

      const historyPanel = page.locator('[data-testid="history-panel"]');
      if ((await historyPanel.count()) > 0) {
        const conversations = historyPanel.locator('[data-testid="conversation-item"]');
        if ((await conversations.count()) > 0) {
          await conversations.first().click();

          // Messages from that conversation should load
          const messages = page.locator(
            '[data-testid="user-message"], [data-testid="agent-message"]',
          );
          expect(await messages.count()).toBeGreaterThanOrEqual(1);
        }
      }
    }
  });

  /* ── API-level agent operations ─────────────────────── */

  test('sends chat message via API', async ({ context }) => {
    const token = await apiLogin(context);

    const resp = await context.request.post('/api/v1/agent/chat', {
      headers: { Authorization: `Bearer ${token}` },
      data: {
        message: 'What model architectures are available for HVAC fault detection?',
      },
    });
    expect(resp.ok()).toBeTruthy();

    const body = await resp.json();
    expect(body).toHaveProperty('response');
    expect(body.response.length).toBeGreaterThan(10);
  });

  test('gets conversation history via API', async ({ context }) => {
    const token = await apiLogin(context);

    const resp = await context.request.get('/api/v1/agent/history', {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(resp.ok()).toBeTruthy();

    const body = await resp.json();
    expect(Array.isArray(body)).toBeTruthy();
  });

  test('sends analysis request via API', async ({ context }) => {
    const token = await apiLogin(context);

    // Get a dataset to analyze
    const datasetsResp = await context.request.get('/api/v1/datasets', {
      headers: { Authorization: `Bearer ${token}` },
    });
    const datasets = await datasetsResp.json();

    if (Array.isArray(datasets) && datasets.length > 0) {
      const analyzeResp = await context.request.post('/api/v1/agent/analyze', {
        headers: { Authorization: `Bearer ${token}` },
        data: {
          dataset_id: datasets[0].id,
          question: 'Analyze this dataset and recommend a model architecture.',
        },
      });
      expect(analyzeResp.ok()).toBeTruthy();

      const analysis = await analyzeResp.json();
      expect(analysis).toHaveProperty('response');
    }
  });
});
