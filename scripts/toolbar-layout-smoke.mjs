// Layout regression without a server or the Tauri runtime. Requires Playwright + Chromium.
// PLAYWRIGHT_MODULE may point to an existing absolute playwright/index.mjs.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
const { chromium } = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
const root = fileURLToPath(new URL('..', import.meta.url));
const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage();
  page.setDefaultTimeout(4000);
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.route('http://gibbon.test/**', route => {
    let file = path.join(root, 'ui', new URL(route.request().url()).pathname);
    if (file.endsWith('/')) file += 'index.html';
    // Test the real Toolbar module, markup and CSS; native app behavior is separate.
    if (/(app.js|telemetry.js)$/.test(file)) return route.fulfill({ body: '', contentType: 'text/javascript' });
    const contentType = file.endsWith('.css') ? 'text/css' : file.endsWith('.js') ? 'text/javascript' : 'text/html';
    return route.fulfill({ body: fs.readFileSync(file), contentType });
  });
  const available = async selector => await page.locator(selector).isVisible() &&
    await page.locator(selector).getAttribute('aria-disabled') !== 'true';
  const assertFocus = async () => {
    const active = await page.evaluate(() => {
      const r = document.activeElement.getBoundingClientRect();
      const b = document.querySelector('#toolbar-items').getBoundingClientRect();
      return {
        id: document.activeElement.id, left: r.left, right: r.right, start: b.left, end: b.right,
        opacity: getComputedStyle(document.querySelector('#controls')).opacity,
      };
    });
    if (!active.id.startsWith('toolbar-')) {
      assert(active.left >= active.start - .1 && active.right <= active.end + .1, JSON.stringify(active));
      assert.equal(active.opacity, '1');
    }
  };
  for (const [width, height] of [[144, 32], [144, 48], [300, 80]]) {
    for (const dev of [false, true]) {
      await page.setViewportSize({ width, height });
      await page.mouse.move(-1, -1);
      await page.goto('http://gibbon.test/');
      await page.evaluate(dev => {
        document.querySelector('#edit').disabled = false;
        const badge = document.querySelector('#dev-instance');
        badge.hidden = !dev;
        badge.textContent = dev ? 'DEV' : '';
      }, dev);
      await page.waitForTimeout(70);
      assert.equal(await page.locator('#controls').evaluate(e => getComputedStyle(e).opacity), '0');
      assert.equal(await page.locator('#controls').evaluate(e => getComputedStyle(e).pointerEvents), 'none');
      const before = await page.locator('#viewport').evaluate(e => [e.clientWidth, e.clientHeight]);
      await page.mouse.move(width / 2, 15);
      const seen = new Set();
      for (let i = 0; i < 8; i++) {
        const track = await page.locator('#toolbar-items').boundingBox();
        for (const button of await page.locator('#toolbar-items button').all()) {
          const box = await button.boundingBox();
          if (box.x >= track.x - .1 && box.x + box.width <= track.x + track.width + .1) {
            assert(box.y >= 0 && box.y + box.height <= height);
            seen.add(await button.getAttribute('id'));
          }
        }
        if (!await available('#toolbar-next')) break;
        await page.locator('#toolbar-next').click();
      }
      assert.equal(seen.size, 8, `${width}×${height} DEV=${dev}: ${[...seen]}`);
      assert.equal(await page.locator('#toolbar-next').isHidden(), width === 300);
      for (let i = 0; i < 7 && await available('#toolbar-previous'); i++) {
        await page.locator('#toolbar-previous').click();
      }
      await page.locator('#fold-toggle').focus();
      for (let i = 0; i < 8; i++) {
        await assertFocus();
        await page.keyboard.press('Tab');
      }
      await page.locator('#close').focus();
      for (let i = 0; i < 8; i++) {
        await assertFocus();
        await page.keyboard.press('Shift+Tab');
      }
      assert.deepEqual(await page.locator('#viewport').evaluate(e => [e.clientWidth, e.clientHeight]), before);
      await page.evaluate(() => document.body.classList.add('folded'));
      await page.waitForTimeout(40);
      assert(await page.locator('#toolbar-next').isHidden());
      const fold = await page.locator('#fold-toggle').boundingBox();
      assert(fold.x >= 0 && fold.x + fold.width <= width);
      await page.evaluate(() => {
        document.body.classList.remove('folded');
        document.body.classList.add('image-note');
      });
      await page.waitForTimeout(40);
      assert(await page.locator('#font-size').isHidden());
      assert(await page.locator('#edit').isHidden());
      console.log(`PASS ${width}×${height} DEV=${dev}: pointer paging, Tab/Shift+Tab, rollover, content size, folded/image layout`);
    }
  }
  assert.deepEqual(errors, []);
} finally {
  await browser.close();
}
