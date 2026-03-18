// =============================================================================
// Prometheus Demo Video Recorder
// Records a full UI walkthrough for hackathon submission using Playwright
//
// Usage: node tools/record-demo.js
// Output: demo-recording/ directory with .webm video
// =============================================================================

const { chromium } = require('playwright');
const path = require('path');

const BASE = 'https://prometheus.automatanexus.com';
const USER = 'DevOps';
const PASS = process.env.PROMETHEUS_DEMO_PASS || 'changeme';

(async () => {
  const browser = await chromium.launch({
    headless: false,
    args: ['--window-size=1920,1080'],
  });

  const context = await browser.newContext({
    viewport: { width: 1920, height: 1080 },
    recordVideo: {
      dir: path.join(__dirname, '..', 'demo-recording'),
      size: { width: 1920, height: 1080 },
    },
  });

  const page = await context.newPage();
  const sleep = (ms) => new Promise(r => setTimeout(r, ms));

  const nav = async (href, label) => {
    console.log(`\n>> ${label}`);
    await page.goto(`${BASE}${href}`, { waitUntil: 'networkidle', timeout: 15000 }).catch(() => {});
    await sleep(2500);
  };

  console.log('=== PROMETHEUS DEMO RECORDING ===\n');

  // ── SCENE 1: Landing Page ──────────────────────────────────────
  console.log('SCENE 1: Landing Page');
  await page.goto(BASE, { waitUntil: 'networkidle' });
  await sleep(4000);

  // Slow scroll through each section
  for (let i = 0; i < 6; i++) {
    await page.evaluate(() => window.scrollBy({ top: 500, behavior: 'smooth' }));
    await sleep(2500);
  }
  // Back to top
  await page.evaluate(() => window.scrollTo({ top: 0, behavior: 'smooth' }));
  await sleep(2000);

  // ── SCENE 2: Login ─────────────────────────────────────────────
  console.log('\nSCENE 2: Login');
  await page.goto(`${BASE}/login`, { waitUntil: 'networkidle' });
  await sleep(2000);

  // Fill login form
  const inputs = await page.$$('input');
  if (inputs.length >= 2) {
    await inputs[0].click();
    await inputs[0].type(USER, { delay: 80 });
    await sleep(500);
    await inputs[1].click();
    await inputs[1].type(PASS, { delay: 60 });
    await sleep(800);
  }

  // Click Sign In
  await page.click('button >> text=Sign In').catch(() =>
    page.click('button >> text=Login').catch(() => {})
  );
  await sleep(4000);

  // ── SCENE 3: Dashboard ─────────────────────────────────────────
  await nav('/dashboard', 'SCENE 3: Dashboard');
  await sleep(2000);

  // ── SCENE 4: Datasets ──────────────────────────────────────────
  await nav('/datasets', 'SCENE 4: Datasets');
  await sleep(2000);

  // Browse Catalog
  console.log('  Opening Browse Catalog...');
  const catalogBtn = await page.$('button >> text=Browse Catalog');
  if (catalogBtn) {
    await catalogBtn.click();
    await sleep(3000);
    // Scroll to see domains
    await page.evaluate(() => {
      const el = document.querySelector('[class*="catalog"], [style*="overflow"]');
      if (el) el.scrollTop += 300;
    });
    await sleep(2000);
    // Close catalog
    const closeBtn = await page.$('button >> text=Close');
    if (closeBtn) { await closeBtn.click(); await sleep(1000); }
  }

  // Connect Source
  console.log('  Opening Connect Source...');
  const connectBtn = await page.$('button >> text=Connect Source');
  if (connectBtn) {
    await connectBtn.click();
    await sleep(3000);
    // Show the source type dropdown and fields
    await sleep(1500);
    // Close
    const cancelConn = await page.$('button >> text=Cancel');
    if (cancelConn) { await cancelConn.click(); await sleep(1000); }
  }

  // Scroll down to show Ingestion Keys
  console.log('  Showing Ingestion Keys...');
  await page.evaluate(() => window.scrollBy({ top: 800, behavior: 'smooth' }));
  await sleep(2000);

  // Create an Ingestion Key
  const createKeyBtn = await page.$('button >> text=Create Key');
  if (createKeyBtn) {
    await createKeyBtn.click();
    await sleep(1000);
    // Fill in key name
    const keyInput = await page.$('input[placeholder*="Key name"]');
    if (keyInput) {
      await keyInput.type('Demo Ingestion Key', { delay: 60 });
      await sleep(800);
    }
    // Click create
    const confirmKey = await page.$('button >> text=Create');
    if (confirmKey) { await confirmKey.click(); await sleep(2500); }
  }

  // Scroll back up
  await page.evaluate(() => window.scrollTo({ top: 0, behavior: 'smooth' }));
  await sleep(1000);

  // Click the HVAC dataset
  console.log('  Opening HVAC dataset...');
  const dsLink = await page.$('text=HVAC_NE_EC');
  if (dsLink) {
    await dsLink.click();
    await sleep(3000);

    // Unlock
    console.log('  Unlocking...');
    const unlockBtn = await page.$('button >> text=Unlock');
    if (unlockBtn) { await unlockBtn.click(); await sleep(2000); }

    // Validate
    console.log('  Validating...');
    const valBtn = await page.$('button >> text=Validate');
    if (valBtn) { await valBtn.click(); await sleep(3000); }

    // Analyze — PrometheusForge AI recommends architectures
    console.log('  Analyzing with Gradient AI...');
    const anaBtn = await page.$('button >> text=Analyze');
    if (anaBtn) {
      await anaBtn.click();
      console.log('  Waiting for Gradient AI recommendations (45s)...');
      await sleep(45000);
      // Scroll to see the recommendation cards
      await page.evaluate(() => window.scrollBy({ top: 400, behavior: 'smooth' }));
      await sleep(3000);

      // Look at the recommendations
      await page.evaluate(() => window.scrollBy({ top: 300, behavior: 'smooth' }));
      await sleep(2000);

      // Click the first recommendation card (LSTM — it's a div, not a button)
      console.log('  Clicking LSTM Autoencoder card...');
      // The cards contain "% match" text — find the first one
      const matchCards = await page.$$('text=% match');
      if (matchCards.length > 0) {
        // Click the parent card of the first match
        const card = await matchCards[0].evaluateHandle(el => {
          // Walk up to find the clickable card div
          let node = el;
          for (let i = 0; i < 5; i++) {
            node = node.parentElement;
            if (node && node.style && node.style.cursor === 'pointer') return node;
            if (node && node.onclick) return node;
          }
          return node;
        });
        if (card) {
          await card.click();
          console.log('  LSTM card selected — training config panel showing');
          await sleep(3000);

          // Scroll to see the training config panel
          await page.evaluate(() => window.scrollBy({ top: 400, behavior: 'smooth' }));
          await sleep(2000);

          // Click "Start Training" button
          const startTrainBtn = await page.$('button >> text=Start Training');
          if (startTrainBtn) {
            await startTrainBtn.click();
            console.log('  Training started!');
            await sleep(4000);
          }
        }
      } else {
        // Fallback: try clicking anything with "lstm" or "LSTM" text
        const lstmEl = await page.$('text=LSTM Autoencoder');
        if (lstmEl) {
          await lstmEl.click();
          await sleep(3000);
          const startTrainBtn = await page.$('button >> text=Start Training');
          if (startTrainBtn) { await startTrainBtn.click(); await sleep(4000); }
        }
      }
    }
  }

  // ── SCENE 5: Training ──────────────────────────────────────────
  await nav('/training', 'SCENE 5: Training');
  await sleep(2000);

  // Show the training list — should see the run we just kicked off
  console.log('  Showing active training run...');
  await sleep(3000);

  // Open Start Training modal to show the UI
  console.log('  Opening Start Training modal...');
  const startBtn = await page.$('button >> text=Start Training');
  if (startBtn) {
    await startBtn.click();
    await sleep(3000);
    // Show the modal (dataset, resume from model, architecture, hyperparams)
    // Scroll inside modal to show all fields
    await sleep(2000);
    const cancelBtn = await page.$('button >> text=Cancel');
    if (cancelBtn) { await cancelBtn.click(); await sleep(1000); }
  }

  // ── SCENE 6: Training Detail (WebSocket live updates) ───────────
  console.log('\n>> SCENE 6: Training Detail (WebSocket live)');
  await nav('/training', 'Finding active training run...');
  await sleep(2000);
  const runLink = await page.$('a[href*="/training/tr_"]');
  if (runLink) {
    await runLink.click();
    console.log('  Connected to WebSocket — watching epochs update live...');
    await sleep(12000);
    await page.evaluate(() => window.scrollBy({ top: 300, behavior: 'smooth' }));
    await sleep(3000);
  }

  // ── SCENE 7: Monitor ──────────────────────────────────────────
  await nav('/monitor', 'SCENE 7: Training Monitor');
  console.log('  Monitor auto-refreshes every 2 seconds...');
  await sleep(6000);

  // ── SCENE 8: Agent Chat ────────────────────────────────────────
  await nav('/agent', 'SCENE 8: PrometheusForge Agent Chat');
  await sleep(1500);
  const chatBox = await page.$('input[placeholder*="Ask"], textarea[placeholder*="Ask"]');
  if (chatBox) {
    await chatBox.click();
    await chatBox.type('What is the best architecture for image classification?', { delay: 40 });
    await sleep(800);
    await page.keyboard.press('Enter');
    console.log('  Waiting for Gradient AI response...');
    await sleep(30000);
    await page.evaluate(() => window.scrollBy({ top: 300, behavior: 'smooth' }));
    await sleep(3000);
  }

  // ── SCENE 9: Models ────────────────────────────────────────────
  await nav('/models', 'SCENE 9: Models');
  await sleep(2000);

  // Click first model
  const modelLink = await page.$('a[href*="/models/mdl"]');
  if (modelLink) {
    await modelLink.click();
    await sleep(3000);
    // Scroll to see architecture info and export options
    await page.evaluate(() => window.scrollBy({ top: 400, behavior: 'smooth' }));
    await sleep(2000);
  }

  // ── SCENE 10: Evaluation ────────────────────────────────────────
  await nav('/evaluation', 'SCENE 10: Evaluation');
  await sleep(2500);

  // ── SCENE 11: Convert ──────────────────────────────────────────
  await nav('/convert', 'SCENE 11: Convert (ONNX / HEF)');
  await sleep(2500);

  // ── SCENE 11: Quantize ─────────────────────────────────────────
  await nav('/quantize', 'SCENE 12: Quantization (Q8/Q4/F16)');
  await sleep(2500);

  // ── SCENE 13: Deployment ───────────────────────────────────────
  await nav('/deployment', 'SCENE 13: Deployment');
  await sleep(1500);

  // Show Add Controller modal
  const addBtn = await page.$('button >> text=Add Controller');
  if (addBtn) {
    await addBtn.click();
    await sleep(2500);
    const cancelDeploy = await page.$('button >> text=Cancel');
    if (cancelDeploy) { await cancelDeploy.click(); await sleep(1000); }
  }

  // ── SCENE 14: Billing ──────────────────────────────────────────
  await nav('/billing', 'SCENE 14: Billing & Subscriptions');
  await sleep(2500);
  // Scroll through tiers
  await page.evaluate(() => window.scrollBy({ top: 500, behavior: 'smooth' }));
  await sleep(2500);
  // Scroll to donation/sponsor section at bottom
  await page.evaluate(() => window.scrollBy({ top: 500, behavior: 'smooth' }));
  await sleep(2500);
  // Scroll to very bottom
  await page.evaluate(() => window.scrollTo({ top: document.body.scrollHeight, behavior: 'smooth' }));
  await sleep(2500);

  // ── SCENE 14: Settings ─────────────────────────────────────────
  await nav('/settings', 'SCENE 15: Profile & Settings');
  await sleep(2500);

  // ── SCENE 16: Admin ────────────────────────────────────────────
  await nav('/admin', 'SCENE 16: Admin Panel');
  await sleep(3000);

  // ── SCENE 17: Header Dropdown ──────────────────────────────────
  console.log('\n>> SCENE 17: Header User Dropdown');
  // Click the last button in the header (user menu)
  const headerBtns = await page.$$('header button');
  if (headerBtns.length > 0) {
    await headerBtns[headerBtns.length - 1].click();
    await sleep(2500);
  }

  // ── END ────────────────────────────────────────────────────────
  console.log('\n=== DEMO COMPLETE ===');
  await sleep(2000);

  await context.close();
  await browser.close();

  console.log('\nVideo saved to: demo-recording/');
})();
