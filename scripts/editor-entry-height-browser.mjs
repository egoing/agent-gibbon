// Real editor UI assets; viewport changes simulate native sizing (not a native test).
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
const {chromium} = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
const root = fileURLToPath(new URL('..', import.meta.url));
const output = process.env.GIBBON_SMOKE_OUTPUT || '/tmp/gibbon159-browser';
fs.mkdirSync(output, {recursive: true});
const browser = await chromium.launch({headless: true});
const results = [];
try {
  for (const [width, height] of [[144,32], [300,48], [300,209], [300,210], [300,280]]) {
    const page = await browser.newPage({viewport: {width, height}});
    const errors = [];
    page.on('pageerror', e => errors.push(e.message));
    await page.exposeBinding('nativeEditorHeight', async ({page}) => {
      const current = page.viewportSize();
      await page.setViewportSize({...current, height: Math.max(current.height, 210)});
    });
    await page.addInitScript(() => {
      window.testCalls = [];
      window.testVisible = true;
      window.testNote = {title: '편집 높이 검증', markdown: '# 초안 원문', html: '<h1>초안 원문</h1>', revision: 1, color: '#dfe2dd'};
      window.__TAURI__ = {
        window: {getCurrentWindow: () => ({label: 'note-editor-height-browser'})},
        webviewWindow: {getCurrentWebviewWindow: () => ({label: 'note-editor-height-browser', listen: async () => () => {}})},
        event: {listen: async () => () => {}},
        core: {invoke: async (name, args) => {
          window.testCalls.push({name, args});
          if (name === 'snapshot') return {visible: window.testVisible, note: window.testNote, always_on_top: true};
          if (name === 'fit_note') return true;
          if (name === 'enter_note_editor') return window.nativeEditorHeight();
          if (name === 'poll_fold' || name === 'poll_warp') return false;
          if (name === 'save_note') {
            window.testNote = {...window.testNote, markdown: args.markdown, html: '<p>저장 완료</p>', revision: window.testNote.revision + 1};
            return window.testNote;
          }
        }},
      };
    });
    await page.route('http://gibbon.test/**', route => {
      const name = path.basename(new URL(route.request().url()).pathname);
      if (['telemetry.js', 'updater-guard.js', 'memo-focus.js'].includes(name)) return route.fulfill({body: '', contentType: 'text/javascript'});
      return route.fulfill({body: fs.readFileSync(path.join(root,'ui',name)), contentType: /\.m?js$/.test(name) ? 'text/javascript' : name.endsWith('.css') ? 'text/css' : 'text/html'});
    });
    await page.goto('http://gibbon.test/index.html');
    // Keyboard navigation also pages the small toolbar without native pointer input.
    await page.locator('#edit').focus();
    await page.keyboard.press('Enter');
    await page.waitForFunction(() => innerHeight >= 210 && document.activeElement.id === 'markdown');
    const box = await page.locator('#markdown').boundingBox();
    assert(box.height > 100 && box.y >= 0 && box.y + box.height <= Math.max(height,210));
    assert.equal(page.viewportSize().width, width);
    assert.equal(page.viewportSize().height, Math.max(height,210));
    await page.locator('#markdown').fill('보존할 초안');
    await page.setViewportSize({width, height: 96});
    await page.evaluate(() => { window.testVisible = false; });
    await page.waitForTimeout(400);
    await page.evaluate(() => { window.testVisible = true; });
    await page.waitForTimeout(450);
    assert.equal(page.viewportSize().height, 96);
    assert.equal(await page.locator('#markdown').inputValue(), '보존할 초안');
    assert.equal(await page.evaluate(() => window.testCalls.filter(c => c.name === 'enter_note_editor').length), 1);
    await page.keyboard.press('Escape');
    await page.locator('#edit').focus();
    await page.keyboard.press('Enter');
    await page.waitForFunction(() => innerHeight === 210);
    await page.locator('#markdown').fill('저장할 초안');
    await page.keyboard.press('Control+s');
    await page.waitForFunction(() => document.getElementById('editor').hidden);
    assert.equal(await page.evaluate(() => window.testNote.markdown), '저장할 초안');
    // Re-enter for an exported screenshot of the actual layout.
    await page.locator('#edit').focus();
    await page.keyboard.press('Enter');
    await page.screenshot({path: path.join(output, `${width}x${height}-editor.png`)});
    assert.deepEqual(errors, []);
    results.push({width, initialHeight: height, editorRect: box, errors});
    await page.close();
  }
  console.log(JSON.stringify({browser: browser.version(), nativeSizing: 'simulated', results, output}));
} finally { await browser.close(); }
