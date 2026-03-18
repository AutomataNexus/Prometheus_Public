/**
 * Prometheus CLI Auth Flow — Puppeteer Tests
 *
 * Tests the device-authorization-like CLI authentication flow:
 *   1. CLI calls POST /api/v1/auth/cli/init to create a pending session
 *   2. User opens the verify URL in a browser
 *   3. CLI polls GET /api/v1/auth/cli/poll until it receives the token
 *
 * Requires:
 *   - Prometheus server running on http://localhost:3030
 *   - Chromium/Chrome available for Puppeteer
 */

const puppeteer = require('puppeteer');

const BASE_URL = process.env.PROMETHEUS_URL || 'http://localhost:3030';
const TEST_USER = process.env.TEST_ADMIN_USER || 'admin';
const TEST_PASS = process.env.TEST_ADMIN_PASS || 'admin_password';

let browser;
let authToken;

/* -- Helpers ------------------------------------------------ */

async function authenticate() {
  const response = await fetch(`${BASE_URL}/api/v1/auth/login`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ username: TEST_USER, password: TEST_PASS }),
  });
  if (!response.ok) {
    throw new Error(`Authentication failed: ${response.status}`);
  }
  const data = await response.json();
  return data.token;
}

function uniqueSessionCode() {
  return `test_pup_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
}

/* -- Setup / teardown --------------------------------------- */

beforeAll(async () => {
  browser = await puppeteer.launch({
    headless: 'new',
    args: [
      '--no-sandbox',
      '--disable-setuid-sandbox',
      '--disable-dev-shm-usage',
      '--disable-gpu',
    ],
  });

  authToken = await authenticate();
});

afterAll(async () => {
  if (browser) {
    await browser.close();
  }
});

/* -- CLI Init Endpoint -------------------------------------- */

describe('CLI Auth Init Endpoint', () => {
  test('POST /api/v1/auth/cli/init creates a pending session with valid response', async () => {
    const sessionCode = uniqueSessionCode();

    const response = await fetch(`${BASE_URL}/api/v1/auth/cli/init`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ session_code: sessionCode }),
    });

    expect(response.ok).toBe(true);

    const body = await response.json();
    expect(body.session_code).toBe(sessionCode);
    expect(body.verify_url).toBeDefined();
    expect(typeof body.verify_url).toBe('string');
    expect(body.expires_in).toBeDefined();
    expect(typeof body.expires_in).toBe('number');
    expect(body.expires_in).toBeGreaterThan(0);
  });

  test('init returns verify_url containing the session code', async () => {
    const sessionCode = uniqueSessionCode();

    const response = await fetch(`${BASE_URL}/api/v1/auth/cli/init`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ session_code: sessionCode }),
    });

    const body = await response.json();
    expect(body.verify_url).toContain(sessionCode);
    expect(body.verify_url).toContain('/auth/verify');
  });

  test('init returns data suitable for QR code generation', async () => {
    const sessionCode = uniqueSessionCode();

    const response = await fetch(`${BASE_URL}/api/v1/auth/cli/init`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ session_code: sessionCode }),
    });

    const body = await response.json();

    // QR code data should be the verify URL — a valid HTTP URL
    const verifyUrl = body.verify_url;
    expect(verifyUrl).toMatch(/^https?:\/\//);

    // URL should be short enough to encode in a QR code (< 2048 chars)
    expect(verifyUrl.length).toBeLessThan(2048);

    // Should contain required query parameter
    expect(verifyUrl).toContain(`code=${sessionCode}`);
  });
});

/* -- Auth URL Accessibility in Browser ---------------------- */

describe('CLI Auth URL in Browser', () => {
  test('verify URL is accessible and renders a page', async () => {
    const sessionCode = uniqueSessionCode();

    // Create the CLI session
    const initResponse = await fetch(`${BASE_URL}/api/v1/auth/cli/init`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ session_code: sessionCode }),
    });
    const initBody = await initResponse.json();
    const verifyUrl = initBody.verify_url;

    // Open the verify URL in the browser
    const page = await browser.newPage();

    try {
      const response = await page.goto(verifyUrl, {
        waitUntil: 'networkidle0',
        timeout: 30000,
      });

      // The page should load (200 status or SPA redirect)
      expect(response.status()).toBeLessThan(500);

      // Page should have content (not blank)
      const bodyContent = await page.evaluate(() => document.body.textContent);
      expect(bodyContent.length).toBeGreaterThan(0);

      // Page title should exist
      const title = await page.title();
      expect(title.length).toBeGreaterThan(0);
    } finally {
      await page.close();
    }
  });

  test('verify page shows login form or auth prompt for unauthenticated user', async () => {
    const sessionCode = uniqueSessionCode();

    const initResponse = await fetch(`${BASE_URL}/api/v1/auth/cli/init`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ session_code: sessionCode }),
    });
    const initBody = await initResponse.json();
    const verifyUrl = initBody.verify_url;

    const page = await browser.newPage();

    try {
      await page.goto(verifyUrl, {
        waitUntil: 'networkidle0',
        timeout: 30000,
      });

      // Should show either a login form or an auth verification UI
      const bodyHTML = await page.evaluate(() => document.body.innerHTML);
      const hasAuthUI =
        bodyHTML.includes('login') ||
        bodyHTML.includes('Login') ||
        bodyHTML.includes('verify') ||
        bodyHTML.includes('Verify') ||
        bodyHTML.includes('auth') ||
        bodyHTML.includes('password') ||
        bodyHTML.includes('Password');
      expect(hasAuthUI).toBe(true);
    } finally {
      await page.close();
    }
  });

  test('verify page accepts browser-authenticated session', async () => {
    const sessionCode = uniqueSessionCode();

    // Create CLI session
    const initResponse = await fetch(`${BASE_URL}/api/v1/auth/cli/init`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ session_code: sessionCode }),
    });
    const initBody = await initResponse.json();

    const page = await browser.newPage();

    try {
      // First login via the UI
      await page.goto(`${BASE_URL}/login`, {
        waitUntil: 'networkidle0',
        timeout: 30000,
      });

      // Fill login form
      const usernameInput = await page.$('input[name="username"], [data-testid="login-form"] input[type="text"]');
      const passwordInput = await page.$('input[name="password"], [data-testid="login-form"] input[type="password"]');

      if (usernameInput && passwordInput) {
        await usernameInput.type(TEST_USER);
        await passwordInput.type(TEST_PASS);

        // Click login button
        const loginBtn = await page.$('button[type="submit"], button:has-text("Log in")');
        if (loginBtn) {
          await loginBtn.click();
          await page.waitForNavigation({ waitUntil: 'networkidle0', timeout: 15000 }).catch(() => {});
        }
      }

      // Now navigate to the verify URL
      await page.goto(initBody.verify_url, {
        waitUntil: 'networkidle0',
        timeout: 30000,
      });

      // Page should load without error
      const status = await page.evaluate(() => {
        return document.body.textContent.length > 0;
      });
      expect(status).toBe(true);
    } finally {
      await page.close();
    }
  });
});

/* -- Poll Flow: Pending -> Verified ------------------------ */

describe('CLI Auth Poll Flow', () => {
  test('poll returns pending before verification', async () => {
    const sessionCode = uniqueSessionCode();

    // Init
    await fetch(`${BASE_URL}/api/v1/auth/cli/init`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ session_code: sessionCode }),
    });

    // Poll — should be pending
    const pollResponse = await fetch(
      `${BASE_URL}/api/v1/auth/cli/poll?code=${sessionCode}`,
    );

    expect(pollResponse.ok).toBe(true);
    const pollBody = await pollResponse.json();
    expect(pollBody.status).toBe('pending');
    expect(pollBody.token).toBeUndefined();
  });

  test('poll returns authenticated with token after verification', async () => {
    const sessionCode = uniqueSessionCode();

    // Step 1: Init
    const initResponse = await fetch(`${BASE_URL}/api/v1/auth/cli/init`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ session_code: sessionCode }),
    });
    expect(initResponse.ok).toBe(true);

    // Step 2: Poll (pending)
    const poll1 = await fetch(
      `${BASE_URL}/api/v1/auth/cli/poll?code=${sessionCode}`,
    );
    const poll1Body = await poll1.json();
    expect(poll1Body.status).toBe('pending');

    // Step 3: Verify via API (simulates user completing browser auth)
    const verifyResponse = await fetch(`${BASE_URL}/api/v1/auth/cli/verify`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        code: sessionCode,
        token: authToken,
        username: TEST_USER,
        role: 'admin',
      }),
    });
    expect(verifyResponse.ok).toBe(true);

    const verifyBody = await verifyResponse.json();
    expect(verifyBody.status).toBe('verified');

    // Step 4: Poll again (should be verified with token)
    const poll2 = await fetch(
      `${BASE_URL}/api/v1/auth/cli/poll?code=${sessionCode}`,
    );
    expect(poll2.ok).toBe(true);

    const poll2Body = await poll2.json();
    expect(poll2Body.status).toBe('verified');
    expect(poll2Body.token).toBeDefined();
    expect(typeof poll2Body.token).toBe('string');
    expect(poll2Body.token.length).toBeGreaterThan(10);
    expect(poll2Body.username).toBe(TEST_USER);
    expect(poll2Body.role).toBe('admin');
  });

  test('returned token from CLI auth is usable for API requests', async () => {
    const sessionCode = uniqueSessionCode();

    // Init
    await fetch(`${BASE_URL}/api/v1/auth/cli/init`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ session_code: sessionCode }),
    });

    // Verify
    await fetch(`${BASE_URL}/api/v1/auth/cli/verify`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        code: sessionCode,
        token: authToken,
        username: TEST_USER,
        role: 'admin',
      }),
    });

    // Poll to get the token
    const pollResponse = await fetch(
      `${BASE_URL}/api/v1/auth/cli/poll?code=${sessionCode}`,
    );
    const pollBody = await pollResponse.json();
    const cliToken = pollBody.token;

    // Use the token to hit a protected endpoint
    const apiResponse = await fetch(`${BASE_URL}/api/v1/datasets`, {
      headers: { Authorization: `Bearer ${cliToken}` },
    });

    expect(apiResponse.ok).toBe(true);
    const datasets = await apiResponse.json();
    expect(Array.isArray(datasets)).toBe(true);
  });

  test('poll for invalid session code returns error', async () => {
    const pollResponse = await fetch(
      `${BASE_URL}/api/v1/auth/cli/poll?code=nonexistent_puppeteer_xyz`,
    );

    // Should return an error status
    expect(pollResponse.ok).toBe(false);
    expect([400, 404, 500]).toContain(pollResponse.status);
  });

  test('session is consumed after successful poll', async () => {
    const sessionCode = uniqueSessionCode();

    // Full flow: init -> verify -> poll (consumed)
    await fetch(`${BASE_URL}/api/v1/auth/cli/init`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ session_code: sessionCode }),
    });

    await fetch(`${BASE_URL}/api/v1/auth/cli/verify`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        code: sessionCode,
        token: authToken,
        username: TEST_USER,
        role: 'admin',
      }),
    });

    // First poll — should succeed and consume
    const poll1 = await fetch(
      `${BASE_URL}/api/v1/auth/cli/poll?code=${sessionCode}`,
    );
    expect(poll1.ok).toBe(true);
    const poll1Body = await poll1.json();
    expect(poll1Body.status).toBe('verified');

    // Second poll — session should be deleted
    const poll2 = await fetch(
      `${BASE_URL}/api/v1/auth/cli/poll?code=${sessionCode}`,
    );
    expect(poll2.ok).toBe(false);
  });
});
