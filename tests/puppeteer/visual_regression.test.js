/**
 * Prometheus Visual Regression & Screenshot Tests
 *
 * Captures screenshots of every major page at multiple viewports
 * and performs basic visual assertions (file size, background color,
 * accent color, sidebar dimensions, contrast).
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
const SCREENSHOTS_DIR = path.join(OUTPUT_DIR, 'screenshots');

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

async function openAuthenticatedPage(url, viewport) {
  const page = await browser.newPage();
  if (viewport) {
    await page.setViewport(viewport);
  }
  await page.setExtraHTTPHeaders({
    Authorization: `Bearer ${authToken}`,
  });
  await page.goto(url, { waitUntil: 'networkidle0', timeout: 30000 });
  return page;
}

async function getFirstTrainingRunId() {
  const response = await fetch(`${BASE_URL}/api/v1/training`, {
    headers: { Authorization: `Bearer ${authToken}` },
  });
  const runs = await response.json();
  if (!Array.isArray(runs) || runs.length === 0) return null;
  const completed = runs.find((r) => r.status === 'completed');
  return completed ? completed.id : runs[0].id;
}

async function getFirstModelId() {
  const response = await fetch(`${BASE_URL}/api/v1/models`, {
    headers: { Authorization: `Bearer ${authToken}` },
  });
  const models = await response.json();
  if (!Array.isArray(models) || models.length === 0) return null;
  const ready = models.find((m) => m.status === 'ready');
  return ready ? ready.id : models[0].id;
}

async function getFirstDeploymentId() {
  const response = await fetch(`${BASE_URL}/api/v1/deployments`, {
    headers: { Authorization: `Bearer ${authToken}` },
  });
  const deployments = await response.json();
  if (!Array.isArray(deployments) || deployments.length === 0) return null;
  const deployed = deployments.find((d) => d.status === 'deployed');
  return deployed ? deployed.id : deployments[0].id;
}

async function getFirstEvaluationId() {
  const response = await fetch(`${BASE_URL}/api/v1/evaluations`, {
    headers: { Authorization: `Bearer ${authToken}` },
  });
  const evals = await response.json();
  if (!Array.isArray(evals) || evals.length === 0) return null;
  return evals[0].id;
}

/**
 * Sample a pixel from the page and return its RGBA value.
 * x, y are page coordinates.
 */
async function samplePixelColor(page, x, y) {
  return page.evaluate(
    (px, py) => {
      const canvas = document.createElement('canvas');
      canvas.width = 1;
      canvas.height = 1;
      // Use elementFromPoint to find the element, then getComputedStyle
      const el = document.elementFromPoint(px, py);
      if (!el) return null;
      const style = window.getComputedStyle(el);
      return style.backgroundColor;
    },
    x,
    y,
  );
}

/* ── Setup / teardown ─────────────────────────────────── */

beforeAll(async () => {
  if (!fs.existsSync(SCREENSHOTS_DIR)) {
    fs.mkdirSync(SCREENSHOTS_DIR, { recursive: true });
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

/* ── Dashboard screenshots at multiple viewports ──────── */

describe('Dashboard Screenshots', () => {
  test('screenshot dashboard at 1920x1080 (full HD)', async () => {
    const page = await openAuthenticatedPage(`${BASE_URL}/`, {
      width: 1920,
      height: 1080,
    });

    try {
      const screenshotPath = path.join(SCREENSHOTS_DIR, 'dashboard-1920x1080.png');
      await page.screenshot({ path: screenshotPath, fullPage: true });
      expect(fs.existsSync(screenshotPath)).toBe(true);
      const stats = fs.statSync(screenshotPath);
      expect(stats.size).toBeGreaterThan(5000);
    } finally {
      await page.close();
    }
  });

  test('screenshot dashboard at 1280x720 (HD)', async () => {
    const page = await openAuthenticatedPage(`${BASE_URL}/`, {
      width: 1280,
      height: 720,
    });

    try {
      const screenshotPath = path.join(SCREENSHOTS_DIR, 'dashboard-1280x720.png');
      await page.screenshot({ path: screenshotPath, fullPage: true });
      expect(fs.existsSync(screenshotPath)).toBe(true);
      const stats = fs.statSync(screenshotPath);
      expect(stats.size).toBeGreaterThan(5000);
    } finally {
      await page.close();
    }
  });

  test('screenshot dashboard at 375x812 (mobile)', async () => {
    const page = await openAuthenticatedPage(`${BASE_URL}/`, {
      width: 375,
      height: 812,
    });

    try {
      const screenshotPath = path.join(SCREENSHOTS_DIR, 'dashboard-375x812-mobile.png');
      await page.screenshot({ path: screenshotPath, fullPage: true });
      expect(fs.existsSync(screenshotPath)).toBe(true);
      const stats = fs.statSync(screenshotPath);
      expect(stats.size).toBeGreaterThan(5000);
    } finally {
      await page.close();
    }
  });
});

/* ── Page-level screenshots ───────────────────────────── */

describe('Page Screenshots', () => {
  test('screenshot login page', async () => {
    const page = await browser.newPage();
    await page.setViewport({ width: 1280, height: 720 });

    try {
      await page.goto(`${BASE_URL}/login`, {
        waitUntil: 'networkidle0',
        timeout: 30000,
      });

      const screenshotPath = path.join(SCREENSHOTS_DIR, 'login-page.png');
      await page.screenshot({ path: screenshotPath, fullPage: true });
      expect(fs.existsSync(screenshotPath)).toBe(true);
      const stats = fs.statSync(screenshotPath);
      expect(stats.size).toBeGreaterThan(5000);
    } finally {
      await page.close();
    }
  });

  test('screenshot datasets page', async () => {
    const page = await openAuthenticatedPage(`${BASE_URL}/datasets`, {
      width: 1280,
      height: 720,
    });

    try {
      const screenshotPath = path.join(SCREENSHOTS_DIR, 'datasets-page.png');
      await page.screenshot({ path: screenshotPath, fullPage: true });
      expect(fs.existsSync(screenshotPath)).toBe(true);
      const stats = fs.statSync(screenshotPath);
      expect(stats.size).toBeGreaterThan(5000);
    } finally {
      await page.close();
    }
  });

  test('screenshot training detail page with charts', async () => {
    const trainingRunId = await getFirstTrainingRunId();
    if (!trainingRunId) {
      console.warn('No training runs found -- skipping training detail screenshot');
      return;
    }

    const page = await openAuthenticatedPage(
      `${BASE_URL}/training/${trainingRunId}`,
      { width: 1280, height: 720 },
    );

    try {
      const screenshotPath = path.join(SCREENSHOTS_DIR, 'training-detail.png');
      await page.screenshot({ path: screenshotPath, fullPage: true });
      expect(fs.existsSync(screenshotPath)).toBe(true);
      const stats = fs.statSync(screenshotPath);
      expect(stats.size).toBeGreaterThan(5000);
    } finally {
      await page.close();
    }
  });

  test('screenshot model detail page with metrics', async () => {
    const modelId = await getFirstModelId();
    if (!modelId) {
      console.warn('No models found -- skipping model detail screenshot');
      return;
    }

    const page = await openAuthenticatedPage(`${BASE_URL}/models/${modelId}`, {
      width: 1280,
      height: 720,
    });

    try {
      const screenshotPath = path.join(SCREENSHOTS_DIR, 'model-detail.png');
      await page.screenshot({ path: screenshotPath, fullPage: true });
      expect(fs.existsSync(screenshotPath)).toBe(true);
      const stats = fs.statSync(screenshotPath);
      expect(stats.size).toBeGreaterThan(5000);
    } finally {
      await page.close();
    }
  });

  test('screenshot deployment page', async () => {
    const page = await openAuthenticatedPage(`${BASE_URL}/deployment`, {
      width: 1280,
      height: 720,
    });

    try {
      const screenshotPath = path.join(SCREENSHOTS_DIR, 'deployment-page.png');
      await page.screenshot({ path: screenshotPath, fullPage: true });
      expect(fs.existsSync(screenshotPath)).toBe(true);
      const stats = fs.statSync(screenshotPath);
      expect(stats.size).toBeGreaterThan(5000);
    } finally {
      await page.close();
    }
  });

  test('screenshot evaluation page', async () => {
    const evalId = await getFirstEvaluationId();
    const evalUrl = evalId
      ? `${BASE_URL}/evaluation/${evalId}`
      : `${BASE_URL}/evaluation`;

    const page = await openAuthenticatedPage(evalUrl, {
      width: 1280,
      height: 720,
    });

    try {
      const screenshotPath = path.join(SCREENSHOTS_DIR, 'evaluation-page.png');
      await page.screenshot({ path: screenshotPath, fullPage: true });
      expect(fs.existsSync(screenshotPath)).toBe(true);
      const stats = fs.statSync(screenshotPath);
      expect(stats.size).toBeGreaterThan(5000);
    } finally {
      await page.close();
    }
  });

  test('screenshot agent chat interface', async () => {
    const page = await openAuthenticatedPage(`${BASE_URL}/agent`, {
      width: 1280,
      height: 720,
    });

    try {
      const screenshotPath = path.join(SCREENSHOTS_DIR, 'agent-chat.png');
      await page.screenshot({ path: screenshotPath, fullPage: true });
      expect(fs.existsSync(screenshotPath)).toBe(true);
      const stats = fs.statSync(screenshotPath);
      expect(stats.size).toBeGreaterThan(5000);
    } finally {
      await page.close();
    }
  });

  test('screenshot settings page', async () => {
    const page = await openAuthenticatedPage(`${BASE_URL}/settings`, {
      width: 1280,
      height: 720,
    });

    try {
      const screenshotPath = path.join(SCREENSHOTS_DIR, 'settings-page.png');
      await page.screenshot({ path: screenshotPath, fullPage: true });
      expect(fs.existsSync(screenshotPath)).toBe(true);
      const stats = fs.statSync(screenshotPath);
      expect(stats.size).toBeGreaterThan(5000);
    } finally {
      await page.close();
    }
  });
});

/* ── Visual verification ──────────────────────────────── */

describe('Visual Verification', () => {
  test('all captured screenshots are non-empty (> 5KB)', () => {
    const screenshotFiles = fs.readdirSync(SCREENSHOTS_DIR).filter((f) => f.endsWith('.png'));

    if (screenshotFiles.length === 0) {
      console.warn('No screenshots captured yet -- skipping size validation');
      return;
    }

    for (const file of screenshotFiles) {
      const filePath = path.join(SCREENSHOTS_DIR, file);
      const stats = fs.statSync(filePath);
      expect(stats.size).toBeGreaterThan(5000);
    }
  });

  test('NexusEdge cream background color (#FFFDF7) is rendered', async () => {
    const page = await openAuthenticatedPage(`${BASE_URL}/`, {
      width: 1280,
      height: 720,
    });

    try {
      // Sample the main content area background (right of sidebar at ~300px, 400px)
      const bgColor = await page.evaluate(() => {
        // Look for the main content area or body
        const main =
          document.querySelector('.main-content') ||
          document.querySelector('main') ||
          document.body;
        return window.getComputedStyle(main).backgroundColor;
      });

      // The cream #FFFDF7 = rgb(255, 253, 247) - accept either exact or close match
      // Also accept if it's applied to a parent/child element
      const allBgColors = await page.evaluate(() => {
        const elements = document.querySelectorAll('*');
        const colors = new Set();
        for (const el of elements) {
          const bg = window.getComputedStyle(el).backgroundColor;
          if (bg && bg !== 'rgba(0, 0, 0, 0)' && bg !== 'transparent') {
            colors.add(bg);
          }
        }
        return Array.from(colors);
      });

      // #FFFDF7 = rgb(255, 253, 247)
      const hasCreamBg = allBgColors.some(
        (c) => c.includes('255, 253, 247') || c.includes('255,253,247'),
      );
      expect(hasCreamBg).toBe(true);
    } finally {
      await page.close();
    }
  });

  test('teal accent (#14b8a6) appears in buttons or links', async () => {
    const page = await openAuthenticatedPage(`${BASE_URL}/`, {
      width: 1280,
      height: 720,
    });

    try {
      const hasTealAccent = await page.evaluate(() => {
        const elements = document.querySelectorAll('button, a, .btn, .badge, [class*="primary"]');
        for (const el of elements) {
          const styles = window.getComputedStyle(el);
          const bg = styles.backgroundColor;
          const color = styles.color;
          const borderColor = styles.borderColor;
          const allStyles = `${bg} ${color} ${borderColor}`;
          // #14b8a6 = rgb(20, 184, 166)
          if (
            allStyles.includes('20, 184, 166') ||
            allStyles.includes('20,184,166')
          ) {
            return true;
          }
        }
        // Also check via raw CSS custom properties or inline styles
        const allElements = document.querySelectorAll('*');
        for (const el of allElements) {
          const styles = window.getComputedStyle(el);
          const bg = styles.background || '';
          if (bg.includes('20, 184, 166') || bg.includes('20,184,166')) {
            return true;
          }
        }
        return false;
      });

      expect(hasTealAccent).toBe(true);
    } finally {
      await page.close();
    }
  });

  test('sidebar renders with correct width (260px)', async () => {
    const page = await openAuthenticatedPage(`${BASE_URL}/`, {
      width: 1280,
      height: 720,
    });

    try {
      const sidebarWidth = await page.evaluate(() => {
        const sidebar = document.querySelector('.sidebar') || document.querySelector('nav');
        if (!sidebar) return null;
        const rect = sidebar.getBoundingClientRect();
        return rect.width;
      });

      if (sidebarWidth === null) {
        console.warn('Sidebar element not found -- skipping width check');
        return;
      }

      // Sidebar should be 260px per theme.rs SIDEBAR_WIDTH constant
      expect(sidebarWidth).toBeGreaterThanOrEqual(240);
      expect(sidebarWidth).toBeLessThanOrEqual(280);
    } finally {
      await page.close();
    }
  });

  test('dark text on light background (basic contrast check)', async () => {
    const page = await openAuthenticatedPage(`${BASE_URL}/`, {
      width: 1280,
      height: 720,
    });

    try {
      const contrastResult = await page.evaluate(() => {
        function parseRgb(rgbStr) {
          const match = rgbStr.match(/(\d+),\s*(\d+),\s*(\d+)/);
          if (!match) return null;
          return { r: parseInt(match[1]), g: parseInt(match[2]), b: parseInt(match[3]) };
        }

        function relativeLuminance(rgb) {
          const srgb = [rgb.r / 255, rgb.g / 255, rgb.b / 255];
          const linear = srgb.map((c) =>
            c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4),
          );
          return 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2];
        }

        // Check main content headings/paragraphs for contrast
        const textElements = document.querySelectorAll(
          '.main-content h1, .main-content h2, .main-content p, main h1, main h2, main p',
        );

        const results = [];
        for (const el of textElements) {
          const styles = window.getComputedStyle(el);
          const textColor = parseRgb(styles.color);
          const bgColor = parseRgb(styles.backgroundColor);

          if (textColor && bgColor) {
            const textLum = relativeLuminance(textColor);
            const bgLum = relativeLuminance(bgColor);
            const lighter = Math.max(textLum, bgLum);
            const darker = Math.min(textLum, bgLum);
            const ratio = (lighter + 0.05) / (darker + 0.05);
            results.push({ ratio, text: styles.color, bg: styles.backgroundColor });
          }
        }

        return results;
      });

      // If we got contrast results, verify they meet minimum WCAG AA (3:1 for large text)
      if (contrastResult.length > 0) {
        for (const result of contrastResult) {
          expect(result.ratio).toBeGreaterThanOrEqual(3);
        }
      } else {
        // No measurable elements found -- the page still rendered without error
        console.warn('No text+background pairs measurable for contrast -- skipping');
      }
    } finally {
      await page.close();
    }
  });

  test('page renders without blank white screen', async () => {
    const page = await openAuthenticatedPage(`${BASE_URL}/`, {
      width: 1280,
      height: 720,
    });

    try {
      // Take a screenshot and check it's not a blank white image
      const screenshotBuffer = await page.screenshot();
      // A totally blank 1280x720 white PNG would be very small (< 3KB typically)
      // A real rendered page will be much larger
      expect(screenshotBuffer.length).toBeGreaterThan(5000);

      // Also verify that some DOM content is rendered
      const bodyChildCount = await page.evaluate(
        () => document.body.querySelectorAll('*').length,
      );
      expect(bodyChildCount).toBeGreaterThan(5);
    } finally {
      await page.close();
    }
  });
});
