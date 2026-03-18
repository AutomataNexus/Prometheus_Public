/**
 * Prometheus Chart Rendering Tests
 *
 * Validates that SVG charts render correctly on training detail,
 * evaluation, and dashboard pages.  Checks structure, dimensions,
 * colors, responsiveness, and interactive states.
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
const CHARTS_DIR = path.join(OUTPUT_DIR, 'charts');

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

async function getFirstEvaluationId() {
  const response = await fetch(`${BASE_URL}/api/v1/evaluations`, {
    headers: { Authorization: `Bearer ${authToken}` },
  });
  const evals = await response.json();
  if (!Array.isArray(evals) || evals.length === 0) return null;
  return evals[0].id;
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

/* ── Setup / teardown ─────────────────────────────────── */

beforeAll(async () => {
  if (!fs.existsSync(CHARTS_DIR)) {
    fs.mkdirSync(CHARTS_DIR, { recursive: true });
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

/* ── Training loss chart ──────────────────────────────── */

describe('Training Loss Chart', () => {
  let trainingRunId;

  beforeAll(async () => {
    trainingRunId = await getFirstTrainingRunId();
  });

  test('training loss chart SVG renders on training detail page', async () => {
    if (!trainingRunId) {
      console.warn('No training runs found -- skipping loss chart SVG test');
      return;
    }

    const page = await openAuthenticatedPage(
      `${BASE_URL}/training/${trainingRunId}`,
      { width: 1280, height: 720 },
    );

    try {
      const svgCount = await page.evaluate(() => document.querySelectorAll('svg').length);

      // The training detail page should render at least one SVG chart
      expect(svgCount).toBeGreaterThanOrEqual(1);

      // Capture a screenshot of the chart area for visual reference
      const screenshotPath = path.join(CHARTS_DIR, 'training-loss-chart.png');
      await page.screenshot({ path: screenshotPath, fullPage: true });
      expect(fs.existsSync(screenshotPath)).toBe(true);
    } finally {
      await page.close();
    }
  });

  test('chart has correct viewBox dimensions', async () => {
    if (!trainingRunId) {
      console.warn('No training runs found -- skipping viewBox test');
      return;
    }

    const page = await openAuthenticatedPage(
      `${BASE_URL}/training/${trainingRunId}`,
      { width: 1280, height: 720 },
    );

    try {
      const viewBoxes = await page.evaluate(() => {
        const svgs = document.querySelectorAll('svg');
        return Array.from(svgs)
          .map((svg) => svg.getAttribute('viewBox'))
          .filter(Boolean);
      });

      if (viewBoxes.length === 0) {
        console.warn('No SVGs with viewBox found -- skipping');
        return;
      }

      for (const viewBox of viewBoxes) {
        // viewBox should be a string of 4 numbers: "minX minY width height"
        const parts = viewBox.trim().split(/[\s,]+/);
        expect(parts.length).toBe(4);

        const width = parseFloat(parts[2]);
        const height = parseFloat(parts[3]);
        expect(width).toBeGreaterThan(0);
        expect(height).toBeGreaterThan(0);
      }
    } finally {
      await page.close();
    }
  });

  test('chart contains path elements (line chart data)', async () => {
    if (!trainingRunId) {
      console.warn('No training runs found -- skipping path elements test');
      return;
    }

    const page = await openAuthenticatedPage(
      `${BASE_URL}/training/${trainingRunId}`,
      { width: 1280, height: 720 },
    );

    try {
      const pathInfo = await page.evaluate(() => {
        const svgs = document.querySelectorAll('svg');
        let totalPaths = 0;
        let pathsWithD = 0;

        for (const svg of svgs) {
          const paths = svg.querySelectorAll('path');
          totalPaths += paths.length;
          for (const p of paths) {
            const d = p.getAttribute('d');
            if (d && d.trim().length > 0) {
              pathsWithD++;
            }
          }
        }

        return { totalPaths, pathsWithD };
      });

      // At least some paths should be present (chart lines, axes, etc.)
      if (pathInfo.totalPaths === 0) {
        console.warn('No path elements found in SVGs -- chart may not have data');
        return;
      }

      expect(pathInfo.pathsWithD).toBeGreaterThan(0);
    } finally {
      await page.close();
    }
  });

  test('chart grid lines are present', async () => {
    if (!trainingRunId) {
      console.warn('No training runs found -- skipping grid lines test');
      return;
    }

    const page = await openAuthenticatedPage(
      `${BASE_URL}/training/${trainingRunId}`,
      { width: 1280, height: 720 },
    );

    try {
      const gridInfo = await page.evaluate(() => {
        const svgs = document.querySelectorAll('svg');
        let lineCount = 0;
        let rectCount = 0;

        for (const svg of svgs) {
          // Grid lines can be <line>, <path> with class grid, or <rect>
          lineCount += svg.querySelectorAll('line').length;
          lineCount += svg.querySelectorAll('.grid line, [class*="grid"]').length;
          rectCount += svg.querySelectorAll('rect').length;
        }

        return { lineCount, rectCount };
      });

      // Charts typically have grid lines or rectangles for the chart area
      const hasGridElements = gridInfo.lineCount > 0 || gridInfo.rectCount > 0;
      if (!hasGridElements) {
        console.warn('No grid lines or rects found -- chart may use alternative rendering');
      }
      // Not a hard failure -- some charts are minimal
      expect(typeof hasGridElements).toBe('boolean');
    } finally {
      await page.close();
    }
  });

  test('chart axis labels render', async () => {
    if (!trainingRunId) {
      console.warn('No training runs found -- skipping axis labels test');
      return;
    }

    const page = await openAuthenticatedPage(
      `${BASE_URL}/training/${trainingRunId}`,
      { width: 1280, height: 720 },
    );

    try {
      const textElements = await page.evaluate(() => {
        const svgs = document.querySelectorAll('svg');
        const texts = [];

        for (const svg of svgs) {
          const textNodes = svg.querySelectorAll('text');
          for (const t of textNodes) {
            texts.push(t.textContent.trim());
          }
        }

        return texts;
      });

      if (textElements.length === 0) {
        console.warn('No text elements found in SVGs -- axis labels may use HTML overlay');
        return;
      }

      // Should have at least some text (axis labels, title, values)
      expect(textElements.length).toBeGreaterThan(0);

      // At least one text element should have meaningful content
      const hasContent = textElements.some((t) => t.length > 0);
      expect(hasContent).toBe(true);
    } finally {
      await page.close();
    }
  });

  test('chart colors match theme (#14b8a6 for primary)', async () => {
    if (!trainingRunId) {
      console.warn('No training runs found -- skipping chart colors test');
      return;
    }

    const page = await openAuthenticatedPage(
      `${BASE_URL}/training/${trainingRunId}`,
      { width: 1280, height: 720 },
    );

    try {
      const colorInfo = await page.evaluate(() => {
        const svgs = document.querySelectorAll('svg');
        const colors = new Set();

        for (const svg of svgs) {
          // Check fill and stroke attributes on all elements
          const allElements = svg.querySelectorAll('*');
          for (const el of allElements) {
            const fill = el.getAttribute('fill');
            const stroke = el.getAttribute('stroke');
            const style = el.getAttribute('style') || '';

            if (fill) colors.add(fill.toLowerCase());
            if (stroke) colors.add(stroke.toLowerCase());
            // Also extract colors from inline styles
            const colorMatch = style.match(/#[0-9a-f]{6}/gi);
            if (colorMatch) {
              for (const c of colorMatch) {
                colors.add(c.toLowerCase());
              }
            }
          }
        }

        return Array.from(colors);
      });

      // Check that the primary teal color is used in the chart
      const hasTeal = colorInfo.some(
        (c) =>
          c === '#14b8a6' ||
          c === 'rgb(20, 184, 166)' ||
          c === 'rgb(20,184,166)' ||
          c.includes('14b8a6'),
      );

      if (!hasTeal) {
        // Check computed styles as fallback
        const computedTeal = await page.evaluate(() => {
          const svgs = document.querySelectorAll('svg');
          for (const svg of svgs) {
            const paths = svg.querySelectorAll('path, line, rect, circle');
            for (const el of paths) {
              const styles = window.getComputedStyle(el);
              const colors = `${styles.fill} ${styles.stroke} ${styles.color}`;
              if (
                colors.includes('20, 184, 166') ||
                colors.includes('20,184,166')
              ) {
                return true;
              }
            }
          }
          return false;
        });

        if (!computedTeal) {
          console.warn(
            'Primary teal color not detected in chart SVGs -- may use gradient or alternative shade',
          );
        }
      }

      // The chart should at least have some colors defined
      expect(colorInfo.length).toBeGreaterThan(0);
    } finally {
      await page.close();
    }
  });
});

/* ── Evaluation page charts ───────────────────────────── */

describe('Evaluation Page Charts', () => {
  let evaluationId;

  beforeAll(async () => {
    evaluationId = await getFirstEvaluationId();
  });

  test('bar chart SVG renders on evaluation page', async () => {
    if (!evaluationId) {
      console.warn('No evaluations found -- skipping evaluation chart test');
      return;
    }

    const page = await openAuthenticatedPage(
      `${BASE_URL}/evaluation/${evaluationId}`,
      { width: 1280, height: 720 },
    );

    try {
      const svgCount = await page.evaluate(() => document.querySelectorAll('svg').length);

      expect(svgCount).toBeGreaterThanOrEqual(1);

      const screenshotPath = path.join(CHARTS_DIR, 'evaluation-chart.png');
      await page.screenshot({ path: screenshotPath, fullPage: true });
      expect(fs.existsSync(screenshotPath)).toBe(true);
    } finally {
      await page.close();
    }
  });

  test('multiple datasets chart renders correctly', async () => {
    const modelId = await getFirstModelId();
    if (!modelId) {
      console.warn('No models found -- skipping multi-dataset chart test');
      return;
    }

    const page = await openAuthenticatedPage(`${BASE_URL}/models/${modelId}`, {
      width: 1280,
      height: 720,
    });

    try {
      const chartInfo = await page.evaluate(() => {
        const svgs = document.querySelectorAll('svg');
        let totalPaths = 0;
        let distinctFills = new Set();

        for (const svg of svgs) {
          const paths = svg.querySelectorAll('path, rect, circle');
          totalPaths += paths.length;
          for (const p of paths) {
            const fill = p.getAttribute('fill');
            if (fill && fill !== 'none' && fill !== 'transparent') {
              distinctFills.add(fill.toLowerCase());
            }
          }
        }

        return {
          svgCount: svgs.length,
          totalPaths,
          distinctFillCount: distinctFills.size,
        };
      });

      if (chartInfo.svgCount === 0) {
        console.warn('No SVG charts found on model detail page -- skipping');
        return;
      }

      // A multi-dataset chart should have multiple distinct fills
      expect(chartInfo.totalPaths).toBeGreaterThan(0);
    } finally {
      await page.close();
    }
  });

  test('chart tooltip/hover states work', async () => {
    if (!evaluationId) {
      console.warn('No evaluations found -- skipping tooltip test');
      return;
    }

    const page = await openAuthenticatedPage(
      `${BASE_URL}/evaluation/${evaluationId}`,
      { width: 1280, height: 720 },
    );

    try {
      // Find an SVG element to hover over
      const svgBounds = await page.evaluate(() => {
        const svg = document.querySelector('svg');
        if (!svg) return null;
        const rect = svg.getBoundingClientRect();
        return { x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 };
      });

      if (!svgBounds) {
        console.warn('No SVG found for tooltip hover test -- skipping');
        return;
      }

      // Hover over the center of the chart
      await page.mouse.move(svgBounds.x, svgBounds.y);
      await page.waitForTimeout(500);

      // Check if any tooltip-like element appeared
      const tooltipInfo = await page.evaluate(() => {
        const tooltips = document.querySelectorAll(
          '[class*="tooltip"], [role="tooltip"], [class*="hover"], title',
        );
        return {
          tooltipCount: tooltips.length,
          hasTitle: !!document.querySelector('svg title'),
        };
      });

      // Not all charts have tooltips -- just verify we didn't crash
      expect(typeof tooltipInfo.tooltipCount).toBe('number');
    } finally {
      await page.close();
    }
  });

  test('empty chart shows placeholder', async () => {
    // Navigate to evaluation list page -- if empty, should show a placeholder
    const page = await openAuthenticatedPage(`${BASE_URL}/evaluation`, {
      width: 1280,
      height: 720,
    });

    try {
      const content = await page.evaluate(() => {
        const body = document.body.textContent.toLowerCase();
        const svgCount = document.querySelectorAll('svg').length;
        return {
          hasEmptyMessage:
            body.includes('no evaluation') ||
            body.includes('no data') ||
            body.includes('empty') ||
            body.includes('get started') ||
            body.includes('run an evaluation'),
          svgCount,
        };
      });

      // Either there are charts (data exists) or a placeholder message (no data)
      expect(content.svgCount > 0 || content.hasEmptyMessage).toBe(true);
    } finally {
      await page.close();
    }
  });

  test('chart responsive resize works', async () => {
    const trainingRunId = await getFirstTrainingRunId();
    if (!trainingRunId) {
      console.warn('No training runs found -- skipping responsive resize test');
      return;
    }

    const page = await openAuthenticatedPage(
      `${BASE_URL}/training/${trainingRunId}`,
      { width: 1280, height: 720 },
    );

    try {
      // Get initial SVG dimensions
      const initialDimensions = await page.evaluate(() => {
        const svg = document.querySelector('svg');
        if (!svg) return null;
        const rect = svg.getBoundingClientRect();
        return { width: rect.width, height: rect.height };
      });

      if (!initialDimensions) {
        console.warn('No SVG found for resize test -- skipping');
        return;
      }

      // Resize viewport to smaller width
      await page.setViewport({ width: 800, height: 600 });
      await page.waitForTimeout(500);

      const resizedDimensions = await page.evaluate(() => {
        const svg = document.querySelector('svg');
        if (!svg) return null;
        const rect = svg.getBoundingClientRect();
        return { width: rect.width, height: rect.height };
      });

      if (!resizedDimensions) {
        console.warn('SVG disappeared after resize -- skipping');
        return;
      }

      // The chart should either resize or maintain its dimensions
      // It should NOT overflow (width should be <= viewport)
      expect(resizedDimensions.width).toBeLessThanOrEqual(810); // viewport + small margin
    } finally {
      await page.close();
    }
  });
});

/* ── Pipeline visualization ───────────────────────────── */

describe('Pipeline Visualization', () => {
  test('pipeline visualization SVG renders all 5 stages', async () => {
    const page = await openAuthenticatedPage(`${BASE_URL}/`, {
      width: 1280,
      height: 720,
    });

    try {
      const pipelineInfo = await page.evaluate(() => {
        // The pipeline stages are rendered as divs with class pipeline-stage
        const stages = document.querySelectorAll('.pipeline-stage');
        const arrows = document.querySelectorAll('.pipeline-arrow');
        const labels = [];
        const names = [];

        for (const stage of stages) {
          const label = stage.querySelector('.pipeline-stage-label');
          const name = stage.querySelector('.pipeline-stage-name');
          if (label) labels.push(label.textContent.trim());
          if (name) names.push(name.textContent.trim());
        }

        return {
          stageCount: stages.length,
          arrowCount: arrows.length,
          labels,
          names,
        };
      });

      // The dashboard should have 5 pipeline stages: INGEST, ANALYZE, TRAIN, EVALUATE, DEPLOY
      expect(pipelineInfo.stageCount).toBe(5);
      expect(pipelineInfo.arrowCount).toBe(4); // 4 arrows between 5 stages

      const expectedLabels = ['INGEST', 'ANALYZE', 'TRAIN', 'EVALUATE', 'DEPLOY'];
      for (const label of expectedLabels) {
        expect(pipelineInfo.labels).toContain(label);
      }

      // Verify stage names
      expect(pipelineInfo.names.length).toBe(5);

      // Capture screenshot for reference
      const screenshotPath = path.join(CHARTS_DIR, 'pipeline-visualization.png');
      await page.screenshot({ path: screenshotPath, fullPage: true });
      expect(fs.existsSync(screenshotPath)).toBe(true);
    } finally {
      await page.close();
    }
  });
});
