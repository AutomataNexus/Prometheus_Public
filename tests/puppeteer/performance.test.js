/**
 * Prometheus Performance & Load Tests
 *
 * Measures page load times, checks for JavaScript errors,
 * validates resource loading, and monitors memory usage
 * during navigation.
 *
 * Requires:
 *   - Prometheus server running on http://localhost:3030
 *   - Chromium/Chrome available for Puppeteer
 */

const puppeteer = require('puppeteer');
const fs = require('fs');
const path = require('path');

const BASE_URL = process.env.PROMETHEUS_URL || 'http://localhost:3030';
const TEST_USER = process.env.TEST_ADMIN_USER || 'admin';
const TEST_PASS = process.env.TEST_ADMIN_PASS || 'admin_password';

const OUTPUT_DIR = path.resolve(__dirname, 'output');
const PERF_DIR = path.join(OUTPUT_DIR, 'performance');

let browser;
let authToken;

/* ── Helpers ──────────────────────────────────────────── */

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

/**
 * Measures the load time for a given URL.
 * Returns { loadTimeMs, domContentLoaded, resourceCount, consoleErrors }.
 */
async function measurePageLoad(url, opts = {}) {
  const page = await browser.newPage();
  const viewport = opts.viewport || { width: 1280, height: 720 };
  await page.setViewport(viewport);

  if (opts.authenticated !== false) {
    await page.setExtraHTTPHeaders({
      Authorization: `Bearer ${authToken}`,
    });
  }

  const consoleErrors = [];
  page.on('console', (msg) => {
    if (msg.type() === 'error') {
      consoleErrors.push(msg.text());
    }
  });

  const requestFailures = [];
  page.on('requestfailed', (request) => {
    requestFailures.push({
      url: request.url(),
      failure: request.failure()?.errorText || 'unknown',
    });
  });

  const startTime = Date.now();

  try {
    await page.goto(url, {
      waitUntil: 'networkidle0',
      timeout: opts.timeout || 30000,
    });

    const loadTimeMs = Date.now() - startTime;

    const metrics = await page.evaluate(() => {
      const perf = performance.getEntriesByType('navigation')[0];
      return {
        domContentLoaded: perf ? perf.domContentLoadedEventEnd - perf.startTime : null,
        domComplete: perf ? perf.domComplete - perf.startTime : null,
        resourceCount: performance.getEntriesByType('resource').length,
      };
    });

    return {
      loadTimeMs,
      domContentLoaded: metrics.domContentLoaded,
      domComplete: metrics.domComplete,
      resourceCount: metrics.resourceCount,
      consoleErrors,
      requestFailures,
      page,
    };
  } catch (err) {
    await page.close();
    throw err;
  }
}

/* ── Setup / teardown ─────────────────────────────────── */

beforeAll(async () => {
  if (!fs.existsSync(PERF_DIR)) {
    fs.mkdirSync(PERF_DIR, { recursive: true });
  }

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

/* ── Page load performance ────────────────────────────── */

describe('Page Load Performance', () => {
  const perfResults = [];

  afterAll(() => {
    // Write performance results to a JSON file for later analysis
    if (perfResults.length > 0) {
      const reportPath = path.join(PERF_DIR, 'load-times.json');
      fs.writeFileSync(reportPath, JSON.stringify(perfResults, null, 2));
    }
  });

  test('dashboard loads within 5 seconds', async () => {
    const result = await measurePageLoad(`${BASE_URL}/`);

    try {
      perfResults.push({ page: 'dashboard', loadTimeMs: result.loadTimeMs });
      expect(result.loadTimeMs).toBeLessThan(5000);
    } finally {
      await result.page.close();
    }
  });

  test('login page loads within 3 seconds', async () => {
    const result = await measurePageLoad(`${BASE_URL}/login`, {
      authenticated: false,
    });

    try {
      perfResults.push({ page: 'login', loadTimeMs: result.loadTimeMs });
      expect(result.loadTimeMs).toBeLessThan(3000);
    } finally {
      await result.page.close();
    }
  });

  test('dataset list page loads within 5 seconds', async () => {
    const result = await measurePageLoad(`${BASE_URL}/datasets`);

    try {
      perfResults.push({ page: 'datasets', loadTimeMs: result.loadTimeMs });
      expect(result.loadTimeMs).toBeLessThan(5000);
    } finally {
      await result.page.close();
    }
  });

  test('training detail page loads within 5 seconds', async () => {
    // First get a training run ID
    let trainingRunId = null;
    try {
      const resp = await fetch(`${BASE_URL}/api/v1/training`, {
        headers: { Authorization: `Bearer ${authToken}` },
      });
      const runs = await resp.json();
      if (Array.isArray(runs) && runs.length > 0) {
        const completed = runs.find((r) => r.status === 'completed');
        trainingRunId = completed ? completed.id : runs[0].id;
      }
    } catch {
      // no runs available
    }

    if (!trainingRunId) {
      console.warn('No training runs found -- skipping training detail load test');
      return;
    }

    const result = await measurePageLoad(
      `${BASE_URL}/training/${trainingRunId}`,
    );

    try {
      perfResults.push({
        page: 'training-detail',
        loadTimeMs: result.loadTimeMs,
      });
      expect(result.loadTimeMs).toBeLessThan(5000);
    } finally {
      await result.page.close();
    }
  });

  test('navigation between pages completes within 2 seconds', async () => {
    const page = await browser.newPage();
    await page.setViewport({ width: 1280, height: 720 });
    await page.setExtraHTTPHeaders({
      Authorization: `Bearer ${authToken}`,
    });

    try {
      // Navigate to dashboard first
      await page.goto(`${BASE_URL}/`, {
        waitUntil: 'networkidle0',
        timeout: 30000,
      });

      // Measure navigation to datasets page
      const navStart = Date.now();
      await page.goto(`${BASE_URL}/datasets`, {
        waitUntil: 'networkidle0',
        timeout: 10000,
      });
      const navTimeMs = Date.now() - navStart;

      perfResults.push({
        page: 'navigation-dashboard-to-datasets',
        loadTimeMs: navTimeMs,
      });
      expect(navTimeMs).toBeLessThan(2000);

      // Measure navigation to settings page
      const navStart2 = Date.now();
      await page.goto(`${BASE_URL}/settings`, {
        waitUntil: 'networkidle0',
        timeout: 10000,
      });
      const navTimeMs2 = Date.now() - navStart2;

      perfResults.push({
        page: 'navigation-datasets-to-settings',
        loadTimeMs: navTimeMs2,
      });
      expect(navTimeMs2).toBeLessThan(2000);
    } finally {
      await page.close();
    }
  });
});

/* ── Resource loading ─────────────────────────────────── */

describe('Resource Loading', () => {
  test('WASM bundle loads successfully', async () => {
    const page = await browser.newPage();
    await page.setViewport({ width: 1280, height: 720 });
    await page.setExtraHTTPHeaders({
      Authorization: `Bearer ${authToken}`,
    });

    const wasmRequests = [];
    page.on('response', (response) => {
      const url = response.url();
      if (url.endsWith('.wasm') || url.includes('wasm')) {
        wasmRequests.push({
          url,
          status: response.status(),
          ok: response.ok(),
        });
      }
    });

    try {
      await page.goto(`${BASE_URL}/`, {
        waitUntil: 'networkidle0',
        timeout: 30000,
      });

      if (wasmRequests.length === 0) {
        // App may not use WASM -- that's fine for a Leptos/WASM app it might
        // load inline or via a different mechanism
        console.warn(
          'No WASM requests detected -- app may load WASM differently or not use it',
        );
        return;
      }

      // All WASM requests should succeed
      for (const req of wasmRequests) {
        expect(req.ok).toBe(true);
      }
    } finally {
      await page.close();
    }
  });

  test('CSS loads without errors', async () => {
    const page = await browser.newPage();
    await page.setViewport({ width: 1280, height: 720 });
    await page.setExtraHTTPHeaders({
      Authorization: `Bearer ${authToken}`,
    });

    const cssRequests = [];
    page.on('response', (response) => {
      const url = response.url();
      const contentType = response.headers()['content-type'] || '';
      if (url.endsWith('.css') || contentType.includes('text/css')) {
        cssRequests.push({
          url,
          status: response.status(),
          ok: response.ok(),
        });
      }
    });

    try {
      await page.goto(`${BASE_URL}/`, {
        waitUntil: 'networkidle0',
        timeout: 30000,
      });

      // Verify stylesheets loaded in the DOM
      const stylesheetCount = await page.evaluate(
        () => document.styleSheets.length,
      );
      expect(stylesheetCount).toBeGreaterThan(0);

      // All CSS requests should succeed
      for (const req of cssRequests) {
        expect(req.ok).toBe(true);
      }
    } finally {
      await page.close();
    }
  });

  test('no JavaScript console errors on page load', async () => {
    const result = await measurePageLoad(`${BASE_URL}/`);

    try {
      // Filter out non-critical errors (e.g., third-party script warnings)
      const criticalErrors = result.consoleErrors.filter(
        (err) =>
          !err.includes('favicon') &&
          !err.includes('manifest') &&
          !err.includes('service-worker') &&
          !err.includes('DevTools'),
      );

      if (criticalErrors.length > 0) {
        console.warn('Console errors detected:', criticalErrors);
      }

      // Should have zero critical JS errors
      expect(criticalErrors.length).toBe(0);
    } finally {
      await result.page.close();
    }
  });

  test('network requests complete without timeouts', async () => {
    const result = await measurePageLoad(`${BASE_URL}/`);

    try {
      // Filter out expected failures (e.g., analytics, optional resources)
      const criticalFailures = result.requestFailures.filter(
        (f) =>
          !f.url.includes('favicon') &&
          !f.url.includes('analytics') &&
          !f.url.includes('hot-update'),
      );

      if (criticalFailures.length > 0) {
        console.warn('Network request failures:', criticalFailures);
      }

      // No critical network failures should occur
      expect(criticalFailures.length).toBe(0);
    } finally {
      await result.page.close();
    }
  });
});

/* ── Memory usage ─────────────────────────────────────── */

describe('Memory Usage', () => {
  test('memory usage stays reasonable during navigation', async () => {
    const page = await browser.newPage();
    await page.setViewport({ width: 1280, height: 720 });
    await page.setExtraHTTPHeaders({
      Authorization: `Bearer ${authToken}`,
    });

    const memorySnapshots = [];

    try {
      const pages = [
        `${BASE_URL}/`,
        `${BASE_URL}/datasets`,
        `${BASE_URL}/training`,
        `${BASE_URL}/models`,
        `${BASE_URL}/deployment`,
        `${BASE_URL}/evaluation`,
        `${BASE_URL}/agent`,
        `${BASE_URL}/settings`,
      ];

      for (const url of pages) {
        await page.goto(url, { waitUntil: 'networkidle0', timeout: 30000 });

        // Get JS heap usage via performance.measureUserAgentSpecificMemory
        // or fallback to performance.memory (Chrome-specific)
        const memoryInfo = await page.evaluate(() => {
          if (performance.memory) {
            return {
              usedJSHeapSize: performance.memory.usedJSHeapSize,
              totalJSHeapSize: performance.memory.totalJSHeapSize,
              jsHeapSizeLimit: performance.memory.jsHeapSizeLimit,
            };
          }
          return null;
        });

        if (memoryInfo) {
          memorySnapshots.push({
            url,
            usedHeapMB: (memoryInfo.usedJSHeapSize / (1024 * 1024)).toFixed(2),
            totalHeapMB: (memoryInfo.totalJSHeapSize / (1024 * 1024)).toFixed(2),
          });
        }
      }

      // Write memory report
      if (memorySnapshots.length > 0) {
        const reportPath = path.join(PERF_DIR, 'memory-usage.json');
        fs.writeFileSync(reportPath, JSON.stringify(memorySnapshots, null, 2));

        // Memory should not exceed 256MB for any single page
        for (const snapshot of memorySnapshots) {
          expect(parseFloat(snapshot.usedHeapMB)).toBeLessThan(256);
        }

        // Memory should not grow unboundedly -- last page should not be
        // more than 4x the first page's usage (indicates a leak)
        const firstUsage = parseFloat(memorySnapshots[0].usedHeapMB);
        const lastUsage = parseFloat(
          memorySnapshots[memorySnapshots.length - 1].usedHeapMB,
        );

        if (firstUsage > 0) {
          const growthFactor = lastUsage / firstUsage;
          expect(growthFactor).toBeLessThan(4);
        }
      } else {
        console.warn(
          'performance.memory not available -- skipping memory usage assertions',
        );
      }
    } finally {
      await page.close();
    }
  });
});
