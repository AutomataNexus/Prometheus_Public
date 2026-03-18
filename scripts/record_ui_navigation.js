/**
 * Prometheus UI Navigation Video Generator
 *
 * Takes sequential screenshots of every page in the Prometheus UI,
 * then assembles them into an MP4 video using ffmpeg.
 *
 * Usage: node scripts/record_ui_navigation.js
 * Output: assets/ui-navigation.mp4
 */

const puppeteer = require('puppeteer');
const path = require('path');
const fs = require('fs');
const { execSync } = require('child_process');

const BASE_URL = process.env.PROMETHEUS_URL || 'http://localhost:3030';
const FRAME_DIR = path.resolve(__dirname, '../assets/frames');
const OUTPUT_VIDEO = path.resolve(__dirname, '../assets/ui-navigation.mp4');
const OUTPUT_GIF = path.resolve(__dirname, '../assets/ui-navigation.gif');
const VIEWPORT = { width: 1440, height: 900 };

// Frames per "scene" — controls how long each page is shown
const HOLD_FRAMES = 45;  // ~1.5s at 30fps
const TRANSITION_FRAMES = 8; // short pause between transitions

const PAGES = [
  { name: '01-login',            path: '/login',       title: 'Login' },
  { name: '02-dashboard',        path: '/',            title: 'Dashboard' },
  { name: '03-datasets',         path: '/datasets',    title: 'Datasets' },
  { name: '04-training',         path: '/training',    title: 'Training' },
  { name: '05-models',           path: '/models',      title: 'Models' },
  { name: '06-deployment',       path: '/deployment',  title: 'Deployment' },
  { name: '07-evaluation',       path: '/evaluation',  title: 'Evaluation' },
  { name: '08-agent',            path: '/agent',       title: 'AI Agent' },
  { name: '09-settings',         path: '/settings',    title: 'Settings' },
];

async function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function run() {
  // Clean and create frame directory
  if (fs.existsSync(FRAME_DIR)) {
    fs.rmSync(FRAME_DIR, { recursive: true });
  }
  fs.mkdirSync(FRAME_DIR, { recursive: true });

  console.log('Launching browser...');
  const browser = await puppeteer.launch({
    headless: 'new',
    args: [
      '--no-sandbox',
      '--disable-setuid-sandbox',
      '--disable-gpu',
      '--disable-dev-shm-usage',
      `--window-size=${VIEWPORT.width},${VIEWPORT.height}`,
    ],
  });

  const page = await browser.newPage();
  await page.setViewport(VIEWPORT);

  let frameNum = 0;

  function framePath() {
    return path.join(FRAME_DIR, `frame_${String(frameNum).padStart(5, '0')}.png`);
  }

  async function captureHoldFrames(count) {
    const src = framePath();
    await page.screenshot({ path: src, type: 'png' });
    frameNum++;
    // Duplicate the same screenshot for hold duration
    for (let i = 1; i < count; i++) {
      fs.copyFileSync(src, path.join(FRAME_DIR, `frame_${String(frameNum).padStart(5, '0')}.png`));
      frameNum++;
    }
  }

  // Navigate to each page and capture screenshots
  for (const pg of PAGES) {
    console.log(`  Capturing: ${pg.title} (${pg.path})`);
    try {
      await page.goto(`${BASE_URL}${pg.path}`, {
        waitUntil: 'networkidle2',
        timeout: 10000,
      });
    } catch (e) {
      // Page may not fully settle if backend services are down — screenshot anyway
      console.log(`    (timeout on networkidle, capturing current state)`);
    }

    // Give JS time to render
    await sleep(800);

    // Capture the page held for ~1.5s
    await captureHoldFrames(HOLD_FRAMES);

    // If page has scrollable content, scroll down and capture more
    const scrollHeight = await page.evaluate(() => document.body.scrollHeight);
    const viewportHeight = VIEWPORT.height;

    if (scrollHeight > viewportHeight + 200) {
      // Smooth scroll down
      const scrollSteps = 5;
      const scrollPerStep = Math.min((scrollHeight - viewportHeight) / scrollSteps, 400);
      for (let s = 1; s <= scrollSteps; s++) {
        await page.evaluate((y) => window.scrollTo({ top: y, behavior: 'instant' }), scrollPerStep * s);
        await sleep(150);
        await captureHoldFrames(TRANSITION_FRAMES);
      }

      // Hold at bottom
      await captureHoldFrames(HOLD_FRAMES / 2);

      // Scroll back to top
      await page.evaluate(() => window.scrollTo({ top: 0, behavior: 'instant' }));
      await sleep(200);
    }

    // Brief transition pause
    await captureHoldFrames(TRANSITION_FRAMES);
  }

  // Final frame hold on dashboard
  console.log('  Final: return to Dashboard');
  await page.goto(`${BASE_URL}/`, { waitUntil: 'networkidle2', timeout: 10000 }).catch(() => {});
  await sleep(800);
  await captureHoldFrames(HOLD_FRAMES);

  await browser.close();

  const totalFrames = frameNum;
  console.log(`\nCaptured ${totalFrames} frames.`);

  // Assemble video with ffmpeg
  console.log('Assembling MP4 video...');
  try {
    execSync(
      `ffmpeg -y -framerate 30 -i "${FRAME_DIR}/frame_%05d.png" ` +
      `-c:v libx264 -preset medium -crf 23 -pix_fmt yuv420p ` +
      `-vf "scale=1440:900:flags=lanczos" ` +
      `"${OUTPUT_VIDEO}"`,
      { stdio: 'pipe' }
    );
    const videoSize = (fs.statSync(OUTPUT_VIDEO).size / 1024 / 1024).toFixed(1);
    console.log(`Video saved: ${OUTPUT_VIDEO} (${videoSize} MB)`);
  } catch (e) {
    console.error('ffmpeg MP4 failed:', e.stderr?.toString().slice(-200));
  }

  // Also generate a GIF for README embedding
  console.log('Assembling GIF...');
  try {
    execSync(
      `ffmpeg -y -framerate 30 -i "${FRAME_DIR}/frame_%05d.png" ` +
      `-vf "fps=12,scale=720:-1:flags=lanczos,split[s0][s1];[s0]palettegen=max_colors=128[p];[s1][p]paletteuse=dither=bayer" ` +
      `"${OUTPUT_GIF}"`,
      { stdio: 'pipe' }
    );
    const gifSize = (fs.statSync(OUTPUT_GIF).size / 1024 / 1024).toFixed(1);
    console.log(`GIF saved: ${OUTPUT_GIF} (${gifSize} MB)`);
  } catch (e) {
    console.error('ffmpeg GIF failed:', e.stderr?.toString().slice(-200));
  }

  // Cleanup frames
  console.log('Cleaning up frames...');
  fs.rmSync(FRAME_DIR, { recursive: true });

  console.log('\nDone!');
}

run().catch(err => {
  console.error('Fatal error:', err);
  process.exit(1);
});
