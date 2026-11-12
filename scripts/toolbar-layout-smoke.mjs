// Layout regression in Chromium; native window behavior is verified separately.
// PLAYWRIGHT_MODULE and CHROME_EXECUTABLE can point to local installations.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
const { chromium } = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
const root = fileURLToPath(new URL('..', import.meta.url));
const browser = await chromium.launch({ headless: true, ...(process.env.CHROME_EXECUTABLE ? { executablePath: process.env.CHROME_EXECUTABLE } : {}) });
try {
  const page = await browser.newPage();
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.addInitScript(() => { window.__TAURI__ = { core: { invoke: async () => true } }; });
  await page.route('http://gibbon.test/**', route => {
    let file = path.join(root, 'ui', new URL(route.request().url()).pathname);
    if (file.endsWith('/')) file += 'index.html';
    if (/(app|telemetry|terminal|web|web-chrome)\.js$/.test(file)) return route.fulfill({ body: '', contentType: 'text/javascript' });
    const contentType = file.endsWith('.css') ? 'text/css' : file.endsWith('.js') || file.endsWith('.mjs') ? 'text/javascript' : 'text/html';
    return route.fulfill({ body: fs.readFileSync(file), contentType });
  });
  for (const filename of ['index.html', 'terminal.html', 'web.html']) {
    for (const [width, height] of [[144, 80], [300, 100]]) {
      await page.setViewportSize({ width, height });
      await page.goto(`http://gibbon.test/${filename}`);
      await page.waitForFunction(() => typeof window.gibbonToolbarFold === 'function');
      assert.equal(await page.locator('#dev-instance').count(), 0);
      assert.equal(await page.locator('#controls').evaluate(e => getComputedStyle(e).opacity), '1');
      for (const id of ['pin', 'toolbar-toggle', 'close']) {
        assert.equal(await page.locator(`#${id} svg`).count(), 1);
      }
      const row = await page.locator('#controls').boundingBox();
      const content = await page.locator(filename === 'terminal.html' ? '#welcome' : filename === 'web.html' ? 'main' : '#viewport').evaluate(e => e.getBoundingClientRect().top);
      if (filename !== 'terminal.html') assert(content >= row.y + row.height - 1);
      const fold = await page.locator('#toolbar-toggle').boundingBox();
      const close = await page.locator('#close').boundingBox();
      assert(fold.x >= 0 && close.x + close.width <= width);
      await page.locator('#toolbar-toggle').click();
      await page.waitForFunction(() => document.body.classList.contains('toolbar-collapsed'));
      assert.equal(await page.locator('#toolbar-items').isVisible(), false);
      assert.equal(await page.locator('#toolbar-toggle').getAttribute('aria-expanded'), 'false');
      await page.evaluate(() => window.gibbonToolbarFold(false));
      assert.equal(await page.locator('#toolbar-toggle').getAttribute('aria-expanded'), 'true');
      console.log(`PASS ${filename} ${width}×${height}: toolbar row, controls and fold`);
    }
  }
  assert.deepEqual(errors, []);
} finally { await browser.close(); }
