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
      {id: 'cancel', label: '취소', disabled: false},
      {id: 'save', label: '저장', disabled: false, primary: true},
    ]};
    window.__TAURI__ = {webviewWindow: {getCurrentWebviewWindow: () => ({listen: async () => {}})}, core: {invoke: async (name, args) => {
      window.testCalls.push({name, args});
      if (name === 'feedback_snapshot') return {view: window.testView, max_width: innerWidth};
      if (name === 'layout_feedback') return {edge: 'below'};
    }}};
  });
  await page.route('http://gibbon.test/**', route => {
    const name = path.basename(new URL(route.request().url()).pathname);
    if (['app.js', 'telemetry.js', 'updater-guard.js', 'memo-focus.js'].includes(name)) return route.fulfill({body: '', contentType: 'text/javascript'});
    return route.fulfill({body: fs.readFileSync(path.join(root, 'ui', name)), contentType: /\.m?js$/.test(name) ? 'text/javascript' : name.endsWith('.css') ? 'text/css' : 'text/html'});
  });
  for (const width of [144, 300]) {
    await page.setViewportSize({width, height: 240});
    await page.goto('http://gibbon.test/feedback.html');
    await page.waitForSelector('button[data-action="save"]');
    assert.equal(await page.locator('button').count(), 2);
    const layout = await page.locator('button').evaluateAll(buttons => buttons.map(b => {
      const r = b.getBoundingClientRect(); return {x: r.x, right: r.right, bottom: r.bottom};
    }));
    assert(layout.every(r => r.x >= 0 && r.right <= width && r.bottom <= 240));
    for (const action of ['cancel', 'save']) {
      await page.keyboard.press('Tab');
      assert.equal(await page.evaluate(() => document.activeElement.dataset.action), action);
    }
    assert.match(await page.locator('[data-action="save"]').getAttribute('aria-label'), /저장 · (Command|Ctrl) S/);
    await page.keyboard.press('Control+s');
    assert.equal(await page.evaluate(() => window.testCalls.filter(c => c.name === 'feedback_action').at(-1).args.action), 'save-shortcut');
    await page.evaluate(() => { for (const extra of [{isComposing: true}, {repeat: true}, {altKey: true}, {shiftKey: true}]) document.dispatchEvent(new KeyboardEvent('keydown', {key: 's', ctrlKey: true, ...extra})); });
    assert.equal(await page.evaluate(() => window.testCalls.filter(c => c.name === 'feedback_action').length), 1);
    await page.screenshot({path: path.join(output, `feedback-${width}.png`)});
    // A notice refresh reconciles buttons without losing keyboard focus.
    await page.locator('[data-action="save"]').focus();
    await page.evaluate(() => { window.testView.message = '原本ファイル / 원본 파일이 변경되었습니다. 초안을 확인하세요.'; window.testView.error = true; });
    await page.waitForFunction(() => document.querySelector('#message').textContent.includes('초안'));
    assert.equal(await page.evaluate(() => document.activeElement.dataset.action), 'save');
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true);
  }
  for (const width of [144, 300, 600]) {
    await page.setViewportSize({width, height: 240});
    await page.goto('http://gibbon.test/index.html');
    await page.evaluate(() => {
      document.body.classList.add('editing');
      document.querySelector('#content').hidden = true;
      document.querySelector('#editor').hidden = false;
      document.querySelector('#export').hidden = true;
      document.querySelector('#markdown').value = '#164 편집 화면 밀착';
    });
    const geometry = await page.evaluate(() => {
      const row = document.querySelector('#controls').getBoundingClientRect();
      const editor = document.querySelector('#markdown').getBoundingClientRect();
      return {height: row.height, gap: editor.top - row.bottom};
    });
    assert.deepEqual(geometry, {height: 22, gap: 0});
    assert.equal(await page.locator('#export').isVisible(), false);
    await page.screenshot({path: path.join(output, `editor-${width}.png`)});
    const textarea = page.locator('#markdown');
    const bar = page.locator('#memo-scrollbar');
    await textarea.fill(Array.from({length: 120}, (_, i) => `편집 ${i}번째 줄`).join('\n'));
    await page.waitForFunction(() => !document.querySelector('#memo-scrollbar').hidden);
    assert.equal(await bar.getAttribute('aria-controls'), 'markdown');
    await textarea.press('Control+End');
    await page.waitForFunction(() => document.querySelector('#markdown').scrollTop > 0);
    await textarea.press('End');
    await textarea.type(' 커서 뒤 입력');
    assert.match(await textarea.inputValue(), /커서 뒤 입력/);
    assert.equal(await textarea.evaluate(e => e.selectionEnd), (await textarea.inputValue()).length);
    await bar.focus();
    await bar.press('Home');
    await page.waitForFunction(() => document.querySelector('#markdown').scrollTop === 0);
    const bounds = await bar.boundingBox();
    await page.mouse.move(bounds.x + 1, bounds.y + 2);
    await page.mouse.down();
    await page.mouse.move(bounds.x + 1, bounds.y + bounds.height - 2, {steps: 8});
    await page.mouse.up();
    await page.waitForFunction(() => document.querySelector('#markdown').scrollTop > 0);
    assert.equal(await page.locator('#viewport').evaluate(e => e.scrollTop), 0);
    await page.emulateMedia({forcedColors: 'active'});
    assert.equal(await bar.evaluate(e => getComputedStyle(e).display), 'none');
    assert.equal(await textarea.evaluate(e => getComputedStyle(e).scrollbarWidth), 'auto');
    await page.emulateMedia({forcedColors: 'none'});
    await page.screenshot({path: path.join(output, `editor-scroll-${width}.png`)});
    await textarea.fill('짧은 글');
    await page.waitForFunction(() => document.querySelector('#memo-scrollbar').hidden);
    await page.evaluate(() => {
      document.body.classList.remove('editing');
      document.querySelector('#editor').hidden = true;
      const content = document.querySelector('#content');
      content.hidden = false;
      content.innerHTML = '<p>읽기 내용</p>'.repeat(100);
    });
    await page.waitForFunction(() => document.querySelector('#memo-scrollbar').getAttribute('aria-controls') === 'viewport' && !document.querySelector('#memo-scrollbar').hidden);
    await bar.focus();
    await bar.press('End');
    await page.waitForFunction(() => document.querySelector('#viewport').scrollTop > 0);
    await page.evaluate(() => document.body.classList.add('image-note'));
    assert.equal(await page.locator('#controls').evaluate(e => e.getBoundingClientRect().height), 30);
  }
  assert.deepEqual(errors, []);
  console.log(JSON.stringify({browser: browser.version(), widths: [144, 300], errors, output}));
} finally { await browser.close(); }
