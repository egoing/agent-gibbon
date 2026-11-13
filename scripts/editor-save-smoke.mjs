// Browser validation for #16; native save dialogs and file writes are separate.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
const { chromium } = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
const root = fileURLToPath(new URL('..', import.meta.url));
const output = process.env.GIBBON_SMOKE_OUTPUT || '/tmp/gibbon16-browser';
fs.mkdirSync(output, { recursive: true });
const browser = await chromium.launch({headless: true});
try {
  const page = await browser.newPage();
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.addInitScript(() => {
    window.testCalls = [];
    window.testView = {message: '', revision: null, color: '#20252e', buttons: [
      {id: 'export', label: '파일 내보내기', disabled: false},
      {id: 'cancel', label: '취소', disabled: false},
      {id: 'save', label: '메모에 반영', disabled: false, primary: true},
    ]};
    window.__TAURI__ = {core: {invoke: async (name, args) => {
      window.testCalls.push({name, args});
      if (name === 'feedback_snapshot') return {view: window.testView, max_width: innerWidth};
      if (name === 'layout_feedback') return {edge: 'below'};
    }}};
  });
  await page.route('http://gibbon.test/**', route => {
    const name = path.basename(new URL(route.request().url()).pathname);
    if (['telemetry.js', 'updater-guard.js', 'memo-focus.js'].includes(name)) return route.fulfill({body: '', contentType: 'text/javascript'});
    return route.fulfill({body: fs.readFileSync(path.join(root, 'ui', name)), contentType: /\.m?js$/.test(name) ? 'text/javascript' : name.endsWith('.css') ? 'text/css' : 'text/html'});
  });
  for (const width of [144, 300]) {
    await page.setViewportSize({width, height: 240});
    await page.goto('http://gibbon.test/feedback.html');
    await page.waitForSelector('button[data-action="save"]');
    assert.equal(await page.locator('button').count(), 3);
    const layout = await page.locator('button').evaluateAll(buttons => buttons.map(b => {
      const r = b.getBoundingClientRect(); return {x: r.x, right: r.right, bottom: r.bottom};
    }));
    assert(layout.every(r => r.x >= 0 && r.right <= width && r.bottom <= 240));
    for (const action of ['export', 'cancel', 'save']) {
      await page.keyboard.press('Tab');
      assert.equal(await page.evaluate(() => document.activeElement.dataset.action), action);
    }
    assert.match(await page.locator('[data-action="save"]').getAttribute('aria-label'), /메모에 반영 · (Command|Ctrl) S/);
    await page.keyboard.press('Control+s');
    assert.equal(await page.evaluate(() => window.testCalls.filter(c => c.name === 'feedback_action').at(-1).args.action), 'save-shortcut');
    await page.evaluate(() => { for (const extra of [{isComposing: true}, {repeat: true}, {altKey: true}, {shiftKey: true}]) document.dispatchEvent(new KeyboardEvent('keydown', {key: 's', ctrlKey: true, ...extra})); });
    assert.equal(await page.evaluate(() => window.testCalls.filter(c => c.name === 'feedback_action').length), 1);
    await page.locator('[data-action="export"]').click();
    assert.equal(await page.evaluate(() => window.testCalls.filter(c => c.name === 'feedback_action').at(-1).args.action), 'export');
    await page.screenshot({path: path.join(output, `feedback-${width}.png`)});
    // A notice refresh reconciles buttons without losing keyboard focus.
    await page.locator('[data-action="save"]').focus();
    await page.evaluate(() => { window.testView.message = '原本ファイル / 원본 파일이 변경되었습니다. 초안을 확인하세요.'; window.testView.error = true; });
    await page.waitForFunction(() => document.querySelector('#message').textContent.includes('초안'));
    assert.equal(await page.evaluate(() => document.activeElement.dataset.action), 'save');
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
  }
  assert.deepEqual(errors, []);
  console.log(JSON.stringify({browser: browser.version(), widths: [144, 300], errors, output}));
} finally { await browser.close(); }
