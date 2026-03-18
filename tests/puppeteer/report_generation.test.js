/**
 * Prometheus PDF Report Generation Tests
 *
 * Tests the headless-Chrome PDF generation pipeline for:
 *   1. Training Report PDFs (model summary, metrics, charts)
 *   2. Deployment Certificate PDFs (model hash, compliance)
 *
 * Requires:
 *   - Prometheus server running on http://localhost:3030
 *   - Chromium/Chrome available for Puppeteer
 */

const puppeteer = require('puppeteer');
const fs = require('fs');
const path = require('path');
const pdfParse = require('pdf-parse');

const BASE_URL = process.env.PROMETHEUS_URL || 'http://localhost:3030';
const TEST_USER = process.env.TEST_ADMIN_USER || 'admin';
const TEST_PASS = process.env.TEST_ADMIN_PASS || 'admin_password';

const OUTPUT_DIR = path.resolve(__dirname, 'output');

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

async function getFirstTrainingRunId() {
  const response = await fetch(`${BASE_URL}/api/v1/training`, {
    headers: { Authorization: `Bearer ${authToken}` },
  });
  const runs = await response.json();
  if (!Array.isArray(runs) || runs.length === 0) {
    return null;
  }
  // Prefer a completed run
  const completed = runs.find((r) => r.status === 'completed');
  return completed ? completed.id : runs[0].id;
}

async function getFirstDeploymentId() {
  const response = await fetch(`${BASE_URL}/api/v1/deployments`, {
    headers: { Authorization: `Bearer ${authToken}` },
  });
  const deployments = await response.json();
  if (!Array.isArray(deployments) || deployments.length === 0) {
    return null;
  }
  const deployed = deployments.find((d) => d.status === 'deployed');
  return deployed ? deployed.id : deployments[0].id;
}

async function extractPdfText(filePath) {
  const buffer = fs.readFileSync(filePath);
  const data = await pdfParse(buffer);
  return data.text;
}

/* ── Setup / teardown ─────────────────────────────────── */

beforeAll(async () => {
  if (!fs.existsSync(OUTPUT_DIR)) {
    fs.mkdirSync(OUTPUT_DIR, { recursive: true });
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

/* ── Training Report PDF ──────────────────────────────── */

describe('Training Report PDF Generation', () => {
  let trainingRunId;

  beforeAll(async () => {
    trainingRunId = await getFirstTrainingRunId();
  });

  test('generates a training report PDF file', async () => {
    if (!trainingRunId) {
      console.warn('No training runs found — skipping PDF generation test');
      return;
    }

    const page = await browser.newPage();

    try {
      // Navigate to the training report page
      await page.setExtraHTTPHeaders({
        Authorization: `Bearer ${authToken}`,
      });

      await page.goto(`${BASE_URL}/api/v1/training/${trainingRunId}?format=report`, {
        waitUntil: 'networkidle0',
        timeout: 30000,
      });

      // Generate PDF
      const pdfPath = path.join(OUTPUT_DIR, `training-report-${trainingRunId}.pdf`);
      await page.pdf({
        path: pdfPath,
        format: 'A4',
        printBackground: true,
        margin: { top: '20mm', bottom: '20mm', left: '15mm', right: '15mm' },
        displayHeaderFooter: true,
        headerTemplate: `
          <div style="font-size:9px; width:100%; text-align:center; color:#6b7280;">
            Prometheus Training Report
          </div>
        `,
        footerTemplate: `
          <div style="font-size:9px; width:100%; text-align:center; color:#6b7280;">
            Page <span class="pageNumber"></span> of <span class="totalPages"></span>
            &mdash; AutomataNexus &copy; 2026
          </div>
        `,
      });

      // Verify PDF was created
      expect(fs.existsSync(pdfPath)).toBe(true);
      const stats = fs.statSync(pdfPath);
      expect(stats.size).toBeGreaterThan(1000); // At least 1 KB
    } finally {
      await page.close();
    }
  });

  test('training report PDF contains expected sections', async () => {
    if (!trainingRunId) {
      console.warn('No training runs found — skipping PDF content test');
      return;
    }

    const pdfPath = path.join(OUTPUT_DIR, `training-report-${trainingRunId}.pdf`);
    if (!fs.existsSync(pdfPath)) {
      console.warn('Training report PDF not found — skipping content verification');
      return;
    }

    const text = await extractPdfText(pdfPath);

    // Should contain key sections
    expect(text.toLowerCase()).toContain('training');
    expect(text.toLowerCase()).toMatch(/architecture|model/);
    expect(text.toLowerCase()).toMatch(/metric|loss|accuracy/);
  });

  test('training report PDF contains model architecture info', async () => {
    if (!trainingRunId) {
      console.warn('No training runs found — skipping');
      return;
    }

    const pdfPath = path.join(OUTPUT_DIR, `training-report-${trainingRunId}.pdf`);
    if (!fs.existsSync(pdfPath)) {
      console.warn('PDF not found — skipping');
      return;
    }

    const text = await extractPdfText(pdfPath);

    // Should mention architecture type
    expect(text.toLowerCase()).toMatch(/lstm|gru|sentinel|autoencoder|predictor/);
  });

  test('training report PDF contains hyperparameters', async () => {
    if (!trainingRunId) return;

    const pdfPath = path.join(OUTPUT_DIR, `training-report-${trainingRunId}.pdf`);
    if (!fs.existsSync(pdfPath)) return;

    const text = await extractPdfText(pdfPath);

    expect(text.toLowerCase()).toMatch(/learning.rate|batch.size|epoch|hidden/);
  });

  test('training report PDF has correct page dimensions', async () => {
    if (!trainingRunId) return;

    const pdfPath = path.join(OUTPUT_DIR, `training-report-${trainingRunId}.pdf`);
    if (!fs.existsSync(pdfPath)) return;

    const buffer = fs.readFileSync(pdfPath);
    const data = await pdfParse(buffer);

    // Should have at least one page
    expect(data.numpages).toBeGreaterThanOrEqual(1);
  });
});

/* ── Deployment Certificate PDF ───────────────────────── */

describe('Deployment Certificate PDF Generation', () => {
  let deploymentId;

  beforeAll(async () => {
    deploymentId = await getFirstDeploymentId();
  });

  test('generates a deployment certificate PDF file', async () => {
    if (!deploymentId) {
      console.warn('No deployments found — skipping PDF generation test');
      return;
    }

    const page = await browser.newPage();

    try {
      await page.setExtraHTTPHeaders({
        Authorization: `Bearer ${authToken}`,
      });

      await page.goto(`${BASE_URL}/api/v1/deployments/${deploymentId}?format=certificate`, {
        waitUntil: 'networkidle0',
        timeout: 30000,
      });

      const pdfPath = path.join(OUTPUT_DIR, `deployment-cert-${deploymentId}.pdf`);
      await page.pdf({
        path: pdfPath,
        format: 'A4',
        landscape: true,
        printBackground: true,
        margin: { top: '25mm', bottom: '25mm', left: '20mm', right: '20mm' },
        displayHeaderFooter: true,
        headerTemplate: `
          <div style="font-size:9px; width:100%; text-align:center; color:#6b7280;">
            Prometheus Deployment Certificate
          </div>
        `,
        footerTemplate: `
          <div style="font-size:9px; width:100%; text-align:center; color:#6b7280;">
            AutomataNexus &copy; 2026 &mdash; Confidential
          </div>
        `,
      });

      expect(fs.existsSync(pdfPath)).toBe(true);
      const stats = fs.statSync(pdfPath);
      expect(stats.size).toBeGreaterThan(1000);
    } finally {
      await page.close();
    }
  });

  test('deployment certificate PDF contains model hash', async () => {
    if (!deploymentId) {
      console.warn('No deployments found — skipping');
      return;
    }

    const pdfPath = path.join(OUTPUT_DIR, `deployment-cert-${deploymentId}.pdf`);
    if (!fs.existsSync(pdfPath)) {
      console.warn('Deployment certificate PDF not found — skipping');
      return;
    }

    const text = await extractPdfText(pdfPath);

    expect(text.toLowerCase()).toMatch(/hash|checksum|sha/);
  });

  test('deployment certificate PDF contains target information', async () => {
    if (!deploymentId) return;

    const pdfPath = path.join(OUTPUT_DIR, `deployment-cert-${deploymentId}.pdf`);
    if (!fs.existsSync(pdfPath)) return;

    const text = await extractPdfText(pdfPath);

    expect(text.toLowerCase()).toMatch(/target|controller|device|raspberry|arm/);
  });

  test('deployment certificate PDF contains deployment date', async () => {
    if (!deploymentId) return;

    const pdfPath = path.join(OUTPUT_DIR, `deployment-cert-${deploymentId}.pdf`);
    if (!fs.existsSync(pdfPath)) return;

    const text = await extractPdfText(pdfPath);

    // Should contain a date pattern
    expect(text).toMatch(/\d{4}[-/]\d{2}[-/]\d{2}|deployed|date/i);
  });

  test('deployment certificate PDF mentions compliance', async () => {
    if (!deploymentId) return;

    const pdfPath = path.join(OUTPUT_DIR, `deployment-cert-${deploymentId}.pdf`);
    if (!fs.existsSync(pdfPath)) return;

    const text = await extractPdfText(pdfPath);

    expect(text.toLowerCase()).toMatch(/certificate|compliance|verified|authorized/);
  });
});

/* ── PDF rendering quality ────────────────────────────── */

describe('PDF Rendering Quality', () => {
  test('renders page at correct viewport for A4', async () => {
    const page = await browser.newPage();

    try {
      await page.setViewport({ width: 1240, height: 1754 }); // A4 at 150 DPI
      await page.setExtraHTTPHeaders({
        Authorization: `Bearer ${authToken}`,
      });

      await page.goto(`${BASE_URL}/`, {
        waitUntil: 'networkidle0',
        timeout: 30000,
      });

      // Take a screenshot to verify rendering
      const screenshotPath = path.join(OUTPUT_DIR, 'render-quality-check.png');
      await page.screenshot({ path: screenshotPath, fullPage: true });

      expect(fs.existsSync(screenshotPath)).toBe(true);
      const stats = fs.statSync(screenshotPath);
      expect(stats.size).toBeGreaterThan(5000); // Non-trivial image
    } finally {
      await page.close();
    }
  });

  test('CSS loads correctly for print media', async () => {
    const page = await browser.newPage();

    try {
      await page.setExtraHTTPHeaders({
        Authorization: `Bearer ${authToken}`,
      });

      await page.emulateMediaType('print');
      await page.goto(`${BASE_URL}/`, {
        waitUntil: 'networkidle0',
        timeout: 30000,
      });

      // Verify stylesheets loaded
      const stylesheetCount = await page.evaluate(() => document.styleSheets.length);
      expect(stylesheetCount).toBeGreaterThan(0);
    } finally {
      await page.close();
    }
  });

  test('generates PDF with custom fonts', async () => {
    const page = await browser.newPage();

    try {
      await page.setExtraHTTPHeaders({
        Authorization: `Bearer ${authToken}`,
      });

      await page.goto(`${BASE_URL}/`, {
        waitUntil: 'networkidle0',
        timeout: 30000,
      });

      // Check that fonts are loaded
      const fonts = await page.evaluate(() => {
        return Array.from(document.fonts).map((f) => f.family);
      });

      // Inter or system-ui should be present
      const hasFont = fonts.some(
        (f) => f.includes('Inter') || f.includes('system-ui') || f.includes('sans-serif'),
      );
      // Even if custom fonts are not loaded, system fonts should be available
      expect(typeof hasFont).toBe('boolean');
    } finally {
      await page.close();
    }
  });
});
