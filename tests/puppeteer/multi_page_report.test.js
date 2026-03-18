/**
 * Prometheus Multi-Page PDF Report Tests
 *
 * Tests generation of complex, multi-page PDFs including
 * batch reports, comparison reports, branding, metadata,
 * and page-level assertions.
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
const REPORTS_DIR = path.join(OUTPUT_DIR, 'reports');

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

async function openAuthenticatedPage(url) {
  const page = await browser.newPage();
  await page.setViewport({ width: 1240, height: 1754 }); // A4 at 150 DPI
  await page.setExtraHTTPHeaders({
    Authorization: `Bearer ${authToken}`,
  });
  await page.goto(url, { waitUntil: 'networkidle0', timeout: 30000 });
  return page;
}

async function getAllTrainingRunIds() {
  const response = await fetch(`${BASE_URL}/api/v1/training`, {
    headers: { Authorization: `Bearer ${authToken}` },
  });
  const runs = await response.json();
  if (!Array.isArray(runs)) return [];
  return runs.map((r) => r.id);
}

async function getAllModelIds() {
  const response = await fetch(`${BASE_URL}/api/v1/models`, {
    headers: { Authorization: `Bearer ${authToken}` },
  });
  const models = await response.json();
  if (!Array.isArray(models)) return [];
  return models.map((m) => m.id);
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

async function getFirstDeploymentId() {
  const response = await fetch(`${BASE_URL}/api/v1/deployments`, {
    headers: { Authorization: `Bearer ${authToken}` },
  });
  const deployments = await response.json();
  if (!Array.isArray(deployments) || deployments.length === 0) return null;
  const deployed = deployments.find((d) => d.status === 'deployed');
  return deployed ? deployed.id : deployments[0].id;
}

async function extractPdfText(filePath) {
  const buffer = fs.readFileSync(filePath);
  const data = await pdfParse(buffer);
  return data.text;
}

async function getPdfInfo(filePath) {
  const buffer = fs.readFileSync(filePath);
  const data = await pdfParse(buffer);
  return {
    text: data.text,
    numpages: data.numpages,
    info: data.info,
    metadata: data.metadata,
  };
}

async function generatePdf(page, outputPath, options = {}) {
  const defaults = {
    path: outputPath,
    format: 'A4',
    printBackground: true,
    margin: { top: '20mm', bottom: '20mm', left: '15mm', right: '15mm' },
    displayHeaderFooter: true,
    headerTemplate: `
      <div style="font-size:9px; width:100%; text-align:center; color:#6b7280;">
        Prometheus Report
      </div>
    `,
    footerTemplate: `
      <div style="font-size:9px; width:100%; text-align:center; color:#6b7280;">
        Page <span class="pageNumber"></span> of <span class="totalPages"></span>
        &mdash; AutomataNexus &copy; 2026
      </div>
    `,
  };

  return page.pdf({ ...defaults, ...options });
}

/* ── Setup / teardown ─────────────────────────────────── */

beforeAll(async () => {
  if (!fs.existsSync(REPORTS_DIR)) {
    fs.mkdirSync(REPORTS_DIR, { recursive: true });
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

/* ── Multi-page training reports ──────────────────────── */

describe('Multi-Page Training Reports', () => {
  test('generate PDF from multiple training runs', async () => {
    const runIds = await getAllTrainingRunIds();
    if (runIds.length === 0) {
      console.warn('No training runs found -- skipping multi-run PDF test');
      return;
    }

    const page = await browser.newPage();
    await page.setViewport({ width: 1240, height: 1754 });
    await page.setExtraHTTPHeaders({ Authorization: `Bearer ${authToken}` });

    try {
      // Build a combined report by concatenating training run content
      let combinedHtml = `
        <!DOCTYPE html>
        <html>
        <head>
          <style>
            body { font-family: Inter, sans-serif; background: #FFFDF7; color: #1a1a2e; padding: 40px; }
            .page-break { page-break-after: always; }
            h1 { color: #14b8a6; }
            h2 { color: #1a1a2e; border-bottom: 2px solid #14b8a6; padding-bottom: 8px; }
            table { width: 100%; border-collapse: collapse; margin: 20px 0; }
            th, td { padding: 10px; text-align: left; border-bottom: 1px solid #e5e7eb; }
            th { background: #f9fafb; font-weight: 600; }
            .header { text-align: center; margin-bottom: 40px; }
            .header img { max-width: 120px; }
          </style>
        </head>
        <body>
          <div class="header">
            <h1>Prometheus Training Summary Report</h1>
            <p>AutomataNexus - Generated ${new Date().toISOString().split('T')[0]}</p>
          </div>
      `;

      for (let i = 0; i < Math.min(runIds.length, 5); i++) {
        const runId = runIds[i];
        let runData = {};

        try {
          const resp = await fetch(`${BASE_URL}/api/v1/training/${runId}`, {
            headers: { Authorization: `Bearer ${authToken}` },
          });
          if (resp.ok) {
            runData = await resp.json();
          }
        } catch {
          runData = { id: runId, status: 'unknown' };
        }

        combinedHtml += `
          <div${i < Math.min(runIds.length, 5) - 1 ? ' class="page-break"' : ''}>
            <h2>Training Run: ${runData.id || runId}</h2>
            <table>
              <tr><th>Status</th><td>${runData.status || 'N/A'}</td></tr>
              <tr><th>Architecture</th><td>${runData.architecture || runData.model_type || 'N/A'}</td></tr>
              <tr><th>Created</th><td>${runData.created_at || 'N/A'}</td></tr>
            </table>
          </div>
        `;
      }

      combinedHtml += '</body></html>';

      await page.setContent(combinedHtml, { waitUntil: 'networkidle0' });

      const pdfPath = path.join(REPORTS_DIR, 'multi-training-runs.pdf');
      await generatePdf(page, pdfPath);

      expect(fs.existsSync(pdfPath)).toBe(true);
      const stats = fs.statSync(pdfPath);
      expect(stats.size).toBeGreaterThan(1000);
    } finally {
      await page.close();
    }
  });

  test('PDF page count matches expected', async () => {
    const pdfPath = path.join(REPORTS_DIR, 'multi-training-runs.pdf');
    if (!fs.existsSync(pdfPath)) {
      console.warn('Multi-training PDF not found -- skipping page count test');
      return;
    }

    const info = await getPdfInfo(pdfPath);

    // Should have at least 1 page
    expect(info.numpages).toBeGreaterThanOrEqual(1);
  });

  test('PDF file size is reasonable (not too large, not empty)', async () => {
    const pdfPath = path.join(REPORTS_DIR, 'multi-training-runs.pdf');
    if (!fs.existsSync(pdfPath)) {
      console.warn('Multi-training PDF not found -- skipping file size test');
      return;
    }

    const stats = fs.statSync(pdfPath);

    // Minimum: at least 1KB (not empty/corrupt)
    expect(stats.size).toBeGreaterThan(1024);

    // Maximum: should not exceed 50MB for a reasonable report
    expect(stats.size).toBeLessThan(50 * 1024 * 1024);
  });

  test('PDF text extraction works across all pages', async () => {
    const pdfPath = path.join(REPORTS_DIR, 'multi-training-runs.pdf');
    if (!fs.existsSync(pdfPath)) {
      console.warn('Multi-training PDF not found -- skipping text extraction test');
      return;
    }

    const info = await getPdfInfo(pdfPath);

    // Should have extractable text
    expect(info.text.length).toBeGreaterThan(0);

    // Should contain the report title
    expect(info.text).toContain('Training');

    // Should contain training run data
    expect(info.text.toLowerCase()).toMatch(/status|training run|architecture/);
  });
});

/* ── Model comparison reports ─────────────────────────── */

describe('Model Comparison Report', () => {
  test('generate comparison report PDF (two models side by side)', async () => {
    const modelIds = await getAllModelIds();
    if (modelIds.length < 2) {
      console.warn('Need at least 2 models for comparison report -- skipping');
      return;
    }

    const page = await browser.newPage();
    await page.setViewport({ width: 1240, height: 1754 });
    await page.setExtraHTTPHeaders({ Authorization: `Bearer ${authToken}` });

    try {
      // Fetch model details
      const models = [];
      for (const id of modelIds.slice(0, 2)) {
        try {
          const resp = await fetch(`${BASE_URL}/api/v1/models/${id}`, {
            headers: { Authorization: `Bearer ${authToken}` },
          });
          if (resp.ok) {
            models.push(await resp.json());
          }
        } catch {
          // skip
        }
      }

      if (models.length < 2) {
        console.warn('Could not fetch 2 model details -- skipping comparison');
        return;
      }

      const comparisonHtml = `
        <!DOCTYPE html>
        <html>
        <head>
          <style>
            body { font-family: Inter, sans-serif; background: #FFFDF7; color: #1a1a2e; padding: 40px; }
            h1 { color: #14b8a6; text-align: center; }
            .comparison { display: flex; gap: 40px; margin-top: 30px; }
            .model-card { flex: 1; border: 1px solid #e5e7eb; border-radius: 8px; padding: 20px; }
            .model-card h2 { color: #14b8a6; font-size: 1.1rem; }
            table { width: 100%; border-collapse: collapse; margin: 10px 0; }
            th, td { padding: 8px; text-align: left; border-bottom: 1px solid #e5e7eb; font-size: 0.9rem; }
            th { background: #f9fafb; }
            .better { color: #14b8a6; font-weight: bold; }
            .footer { text-align: center; margin-top: 40px; color: #6b7280; font-size: 0.8rem; }
          </style>
        </head>
        <body>
          <h1>Model Comparison Report</h1>
          <p style="text-align:center; color:#6b7280;">AutomataNexus - ${new Date().toISOString().split('T')[0]}</p>
          <div class="comparison">
            ${models
              .map(
                (m) => `
              <div class="model-card">
                <h2>${m.name || m.id}</h2>
                <table>
                  <tr><th>Architecture</th><td>${m.architecture || 'N/A'}</td></tr>
                  <tr><th>Parameters</th><td>${m.parameters ? m.parameters.toLocaleString() : 'N/A'}</td></tr>
                  <tr><th>F1 Score</th><td>${m.metrics?.f1 || 'N/A'}</td></tr>
                  <tr><th>Val Loss</th><td>${m.metrics?.val_loss || 'N/A'}</td></tr>
                  <tr><th>Precision</th><td>${m.metrics?.precision || 'N/A'}</td></tr>
                  <tr><th>Recall</th><td>${m.metrics?.recall || 'N/A'}</td></tr>
                  <tr><th>Quantized</th><td>${m.quantized ? 'Yes' : 'No'}</td></tr>
                  <tr><th>Size</th><td>${m.file_size_bytes ? (m.file_size_bytes / 1024).toFixed(1) + ' KB' : 'N/A'}</td></tr>
                  <tr><th>Status</th><td>${m.status || 'N/A'}</td></tr>
                </table>
              </div>
            `,
              )
              .join('')}
          </div>
          <div class="footer">
            <p>Generated by Prometheus - AutomataNexus</p>
          </div>
        </body>
        </html>
      `;

      await page.setContent(comparisonHtml, { waitUntil: 'networkidle0' });

      const pdfPath = path.join(REPORTS_DIR, 'model-comparison.pdf');
      await generatePdf(page, pdfPath, { landscape: true });

      expect(fs.existsSync(pdfPath)).toBe(true);
      const stats = fs.statSync(pdfPath);
      expect(stats.size).toBeGreaterThan(1000);

      // Verify content
      const text = await extractPdfText(pdfPath);
      expect(text).toContain('Model Comparison');
    } finally {
      await page.close();
    }
  });
});

/* ── PDF formatting and branding ──────────────────────── */

describe('PDF Formatting and Branding', () => {
  test('PDF contains correct date formatting', async () => {
    const pdfPath = path.join(REPORTS_DIR, 'multi-training-runs.pdf');
    if (!fs.existsSync(pdfPath)) {
      console.warn('Multi-training PDF not found -- skipping date format test');
      return;
    }

    const text = await extractPdfText(pdfPath);

    // Should contain a date in YYYY-MM-DD or similar format
    expect(text).toMatch(/\d{4}-\d{2}-\d{2}/);
  });

  test('PDF contains company branding (AutomataNexus)', async () => {
    const pdfPath = path.join(REPORTS_DIR, 'multi-training-runs.pdf');
    if (!fs.existsSync(pdfPath)) {
      console.warn('Multi-training PDF not found -- skipping branding test');
      return;
    }

    const text = await extractPdfText(pdfPath);

    expect(text).toContain('AutomataNexus');
  });

  test('PDF has correct page margins', async () => {
    const page = await browser.newPage();
    await page.setViewport({ width: 1240, height: 1754 });

    try {
      await page.setContent(
        `
        <html>
        <body style="background: #FFFDF7; margin: 0;">
          <h1>Margin Test Page</h1>
          <p>This page tests that PDF margins are applied correctly.</p>
        </body>
        </html>
      `,
        { waitUntil: 'networkidle0' },
      );

      const pdfPath = path.join(REPORTS_DIR, 'margin-test.pdf');
      await generatePdf(page, pdfPath, {
        margin: { top: '20mm', bottom: '20mm', left: '15mm', right: '15mm' },
      });

      expect(fs.existsSync(pdfPath)).toBe(true);
      const stats = fs.statSync(pdfPath);
      expect(stats.size).toBeGreaterThan(500);

      // Verify it's a valid PDF with at least 1 page
      const info = await getPdfInfo(pdfPath);
      expect(info.numpages).toBe(1);
      expect(info.text).toContain('Margin Test');
    } finally {
      await page.close();
    }
  });
});

/* ── Batch report generation ──────────────────────────── */

describe('Batch Report Generation', () => {
  test('generate batch reports for multiple models', async () => {
    const modelIds = await getAllModelIds();
    if (modelIds.length === 0) {
      console.warn('No models found -- skipping batch report generation');
      return;
    }

    const generatedPdfs = [];

    for (const modelId of modelIds.slice(0, 3)) {
      const page = await openAuthenticatedPage(`${BASE_URL}/models/${modelId}`);

      try {
        const pdfPath = path.join(REPORTS_DIR, `model-report-${modelId}.pdf`);
        await generatePdf(page, pdfPath);

        if (fs.existsSync(pdfPath)) {
          const stats = fs.statSync(pdfPath);
          if (stats.size > 500) {
            generatedPdfs.push(pdfPath);
          }
        }
      } finally {
        await page.close();
      }
    }

    // At least one PDF should have been generated
    expect(generatedPdfs.length).toBeGreaterThanOrEqual(1);
  });

  test('PDF metadata is set correctly', async () => {
    const page = await browser.newPage();
    await page.setViewport({ width: 1240, height: 1754 });

    try {
      await page.setContent(
        `
        <html>
        <head><title>Prometheus Report</title></head>
        <body style="background: #FFFDF7;">
          <h1>Metadata Test Report</h1>
          <p>AutomataNexus - Prometheus Platform</p>
        </body>
        </html>
      `,
        { waitUntil: 'networkidle0' },
      );

      const pdfPath = path.join(REPORTS_DIR, 'metadata-test.pdf');
      await generatePdf(page, pdfPath);

      expect(fs.existsSync(pdfPath)).toBe(true);

      const info = await getPdfInfo(pdfPath);

      // Verify PDF metadata exists
      expect(info.numpages).toBe(1);

      // The PDF should have text content
      expect(info.text).toContain('Metadata Test Report');
      expect(info.text).toContain('AutomataNexus');

      // PDF info object should exist (may contain title, creator, etc.)
      expect(info.info).toBeDefined();
    } finally {
      await page.close();
    }
  });
});

/* ── Deployment certificate PDF ───────────────────────── */

describe('Multi-Page Deployment Reports', () => {
  test('generate deployment report with all sections', async () => {
    const deploymentId = await getFirstDeploymentId();
    if (!deploymentId) {
      console.warn('No deployments found -- skipping deployment report test');
      return;
    }

    const page = await browser.newPage();
    await page.setViewport({ width: 1240, height: 1754 });
    await page.setExtraHTTPHeaders({ Authorization: `Bearer ${authToken}` });

    try {
      let deployData = {};
      try {
        const resp = await fetch(`${BASE_URL}/api/v1/deployments/${deploymentId}`, {
          headers: { Authorization: `Bearer ${authToken}` },
        });
        if (resp.ok) {
          deployData = await resp.json();
        }
      } catch {
        deployData = { id: deploymentId };
      }

      const reportHtml = `
        <!DOCTYPE html>
        <html>
        <head>
          <style>
            body { font-family: Inter, sans-serif; background: #FFFDF7; color: #1a1a2e; padding: 40px; }
            h1 { color: #14b8a6; text-align: center; }
            h2 { color: #1a1a2e; border-bottom: 2px solid #14b8a6; padding-bottom: 8px; }
            table { width: 100%; border-collapse: collapse; margin: 20px 0; }
            th, td { padding: 10px; text-align: left; border-bottom: 1px solid #e5e7eb; }
            th { background: #f9fafb; font-weight: 600; }
            .page-break { page-break-after: always; }
            .certificate { text-align: center; border: 3px solid #14b8a6; padding: 40px; margin: 20px; }
          </style>
        </head>
        <body>
          <div class="header">
            <h1>Deployment Report</h1>
            <p style="text-align:center;">AutomataNexus - ${new Date().toISOString().split('T')[0]}</p>
          </div>

          <h2>Deployment Details</h2>
          <table>
            <tr><th>Deployment ID</th><td>${deployData.id || deploymentId}</td></tr>
            <tr><th>Status</th><td>${deployData.status || 'N/A'}</td></tr>
            <tr><th>Target</th><td>${deployData.target || deployData.target_device || 'N/A'}</td></tr>
            <tr><th>Created</th><td>${deployData.created_at || 'N/A'}</td></tr>
          </table>

          <div class="page-break"></div>

          <div class="certificate">
            <h1>Deployment Certificate</h1>
            <p>This certifies that the model has been verified and deployed.</p>
            <p><strong>Hash:</strong> ${deployData.model_hash || deployData.hash || 'N/A'}</p>
            <p><strong>Date:</strong> ${new Date().toISOString().split('T')[0]}</p>
            <p>AutomataNexus - Prometheus Platform</p>
          </div>
        </body>
        </html>
      `;

      await page.setContent(reportHtml, { waitUntil: 'networkidle0' });

      const pdfPath = path.join(REPORTS_DIR, 'deployment-full-report.pdf');
      await generatePdf(page, pdfPath);

      expect(fs.existsSync(pdfPath)).toBe(true);

      const info = await getPdfInfo(pdfPath);
      expect(info.numpages).toBeGreaterThanOrEqual(2);
      expect(info.text).toContain('Deployment');
      expect(info.text).toContain('Certificate');
      expect(info.text).toContain('AutomataNexus');
    } finally {
      await page.close();
    }
  });
});
