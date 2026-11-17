// Exercise the real trusted pages with native placement held by the fixture.
// Run after pnpm build:editor. Only the native compositor needs a live test.
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {memoThemeFor} from '../ui/theme.mjs';

const {chromium} = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
const browser = await chromium.launch({headless: true, channel: 'chrome'});
const viewport = {width: 360, height: 240};
const color = '#6baa92';
const results = [];
try {
  for (const kind of ['text', 'image', 'terminal', 'web']) {
    const cases = ['white', 'black', 'color'].map(design => ({design, failFirst: false}));
    if (['terminal', 'image'].includes(kind)) cases.push({design: 'black', failFirst: true});
    for (const {design, failFirst} of cases) {
      const page = await browser.newPage({viewport, deviceScaleFactor: 2});
      const errors = [];
      page.on('pageerror', error => errors.push(error.message));
      await page.addInitScript(({kind, design, color, failFirst}) => {
        localStorage.setItem('agent-gibbon.memo-design', design);
        let failed = false;
        const placement = new Promise(resolve => { window.place = () => resolve(true); });
        const content = new Promise(resolve => { window.prepareContent = resolve; });
        window.framesAtReveal = [];
        const info = {kind: 'terminal', color, welcome: true, always_on_top: true};
        const listen = async () => () => {};
        const win = {label: 'note-fixture', listen};
        window.__TAURI__ = {
          window: {getCurrentWindow: () => win},
          webviewWindow: {getCurrentWebviewWindow: () => win},
          event: {listen},
          core: {Channel: class {}, async invoke(cmd) {
            if (cmd === 'region_frame_placement') return placement;
            if (cmd === 'region_frame_ready') {
              const img = document.querySelector('#content img');
              const root = document.documentElement;
              window.framesAtReveal.push({
                background: getComputedStyle(root).getPropertyValue('--background').trim(),
                error: document.querySelector('#status, #message')?.textContent || '',
                content: kind === 'text' ? !!document.querySelector('#memo-editor')
                  : kind === 'image' ? !!img?.complete && img.naturalWidth === 8
                  : kind === 'terminal' ? !!document.querySelector('#recommendations button')
                  : !!document.querySelector('#web-recent .recent-open'),
                width: innerWidth, height: innerHeight, dpr: devicePixelRatio,
              });
            }
            if (cmd === 'snapshot') {
              if (kind === 'text') await content;
              return {visible: false, note: {title: '첫 화면', color, revision: 1,
                markdown: '', html: '', capture: kind === 'image' ? {width: 8, height: 8} : null}};
            }
            if (cmd === 'note_image') {
              await content;
              if (failFirst && !failed) { failed = true; throw new Error('첫 이미지 조회 실패'); }
              return 'data:image/svg+xml,' + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><rect width="8" height="8" fill="#365"/></svg>');
            }
            if (cmd === 'web_chrome') { await content; return info; }
            if (cmd === 'terminal_info') return info;
            if (cmd === 'terminal_welcome') {
              await content;
              if (failFirst && !failed) { failed = true; throw new Error('첫 터미널 준비 실패'); }
              return {info, home: '/tmp', location: {path: '/tmp', name: 'tmp', parent: '/'},
                choices: [{id: 'shell', label: '셸', command: 'sh'}], folders: []};
            }
            if (cmd === 'web_recent') { await content; return ['https://example.test/']; }
            return false;
          }},
        };
      }, {kind, design, color, failFirst});
      await page.route('http://gibbon.test/**', async route => {
        const path = new URL(route.request().url()).pathname;
        const contentType = /\.m?js$/.test(path) ? 'text/javascript' : path.endsWith('.css') ? 'text/css' : 'text/html';
        await route.fulfill({contentType, body: await readFile(new URL('../ui' + path, import.meta.url))});
      });
      const entry = ['text', 'image'].includes(kind) ? 'index' : kind;
      await page.goto('http://gibbon.test/' + entry + '.html', {waitUntil: 'commit'});
      await page.waitForFunction(() => window.gibbonRegionFrame);
      assert.equal(await page.evaluate(() => window.framesAtReveal.length), 0);
      await page.evaluate(() => window.place());
      // The document can lay out freely while the content request remains pending.
      await page.evaluate(() => new Promise(done => requestAnimationFrame(() => requestAnimationFrame(done))));
      assert.equal(await page.evaluate(() => window.framesAtReveal.length), 0);
      await page.evaluate(() => window.prepareContent());
      await page.waitForFunction(() => window.framesAtReveal.length > 0, undefined, {timeout: 10000});
      await page.evaluate(() => window.gibbonRegionFrame.ready());
      const expected = memoThemeFor(color, {design}).background;
      const frames = await page.evaluate(() => window.framesAtReveal);
      const error = failFirst ? 'Error: ' + (kind === 'image' ? '첫 이미지 조회 실패' : '첫 터미널 준비 실패') : '';
      assert.deepEqual(frames, [{background: expected, error, content: !failFirst, ...viewport, dpr: 2}]);
      if (failFirst && kind === 'image') {
        await page.waitForSelector('#content img');
        assert.equal(await page.evaluate(() => window.framesAtReveal.length), 1);
      }
      assert.deepEqual(errors, []);
      results.push({kind, design, failFirst, ...frames[0]});
      await page.close();
    }
  }
  console.log(JSON.stringify(results, null, 2));
} finally {
  await browser.close();
}
