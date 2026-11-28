// Exercise the shipped pages in Chromium with a local IPC double. The native
// preference store, system menus, PTYs and keyboard IMEs require separate tests.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const {default: puppeteer} = await import(process.env.PUPPETEER_MODULE || 'puppeteer-core');
const root = fileURLToPath(new URL('../ui/', import.meta.url));
const browser = await puppeteer.launch({
  executablePath: process.env.CHROME_BIN || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  headless: true,
});
const errors = [];
const pages = [];
let revision = 0;
async function open(file, width, height) {
  const page = await browser.newPage();
  pages.push(page);
  page.on('pageerror', error => errors.push(String(error)));
  await page.setViewport({width, height});
  await page.setRequestInterception(true);
  page.on('request', request => {
    const url = new URL(request.url());
    if (url.hostname !== 'jajan.test') return request.abort();
    const asset = path.join(root, url.pathname);
    if (!fs.existsSync(asset)) return request.respond({status: 404, body: ''});
    const type = {'.js':'text/javascript', '.mjs':'text/javascript', '.html':'text/html', '.css':'text/css', '.png':'image/png', '.svg':'image/svg+xml'};
    request.respond({contentType: type[path.extname(asset)], body: fs.readFileSync(asset)});
  });
  await page.exposeFunction('saveLanguage', async choice => {
    const snapshot = {choice, locale: choice === 'system' ? 'ja' : choice, revision: ++revision};
    await Promise.all(pages.map(p => p.evaluate(s => window.jajanLanguage.apply(s), snapshot)));
    return snapshot;
  });
  await page.evaluateOnNewDocument(() => {
    const frame = {label:'note-language', listen: async () => () => {}, startResizeDragging: async () => {}};
    window.__TAURI__ = {
      window: {getCurrentWindow: () => frame}, event: {listen: frame.listen},
      webviewWindow: {getCurrentWebviewWindow: () => frame},
      core: {Channel:class {}, invoke: async (name, args) => {
        if (name === 'language_status') {
          await new Promise(resolve => { window.releasePreference = resolve; });
          return {choice:'en', locale:'en', revision:0};
        }
        if (name === 'language_save') {
          if (window.failLanguageSave) throw Error('storage unavailable');
          return window.saveLanguage(args.choice);
        }
        if (name === 'terminal_info') return {kind:'terminal', title:'테마', running:Boolean(window.outputChannel)};
        if (name === 'terminal_welcome') return {info:{kind:'terminal', title:'테마'}, folders:[], choices:[{id:'shell',label:'셸',command:'sh'}], home:'/tmp'};
        if (name === 'terminal_start') {
          if (window.outputChannel) throw Error('unexpected second terminal start');
          window.outputChannel = args.output;
          args.output.onmessage([...new TextEncoder().encode('자동 업데이트')]);
          return true;
        }
        if (name === 'privacy_status') return {terms_accepted:true, notice_ready:true, sharing:false};
        if (name === 'manager_notes') return [];
        if (name === 'skills_list') return {skills:[]};
        if (name === 'updater_status') return {phase:'idle', current_version:'1'};
        if (name === 'autostart_status') return {os_status:'disabled', available:true};
        if (name === 'save_note') return new Promise(() => {}); // Hold the autosave request to exercise a live draft.
        if (name === 'snapshot') return {visible:true, focused:true, folded:false, always_on_top:true,
          note:{title:'자동 업데이트', markdown:'자동 업데이트', html:'<p>자동 업데이트</p>', revision:1, color:'#dfe2dd'}};
        if (name === 'web_chrome') return {welcome:true, always_on_top:true};
        if (name === 'web_recent') return [];
        return false;
      }},
    };
  });
  await page.goto('http://jajan.test/' + file, {waitUntil:'domcontentloaded'});
  assert.equal(await page.evaluate(() => getComputedStyle(document.documentElement).opacity), '0');
  await page.evaluate(() => window.releasePreference());
  await page.waitForFunction(() => document.documentElement.lang === 'en' && !document.documentElement.hasAttribute('data-language-pending'));
  return page;
}
try {
  const manager = await open('manager.html', 600, 360);
  const memo = await open('index.html', 300, 240);
  const web = await open('web.html', 240, 160);
  const term = await open('terminal.html', 300, 240);
  await term.bringToFront();
  await term.waitForFunction(() => !document.getElementById('run-task').disabled);
  await term.click('#run-task');
  await term.waitForFunction(() => terminal.buffer.active.getLine(0)?.translateToString(true) === '자동 업데이트');
  await memo.bringToFront();
  await memo.waitForSelector('#memo-editor[contenteditable=true]');
  await memo.evaluate(() => {
    const editor = document.getElementById('memo-editor');
    editor.focus();
    const selection = getSelection();
    selection.collapse(editor.querySelector('p').firstChild, 2);
    document.execCommand('insertText', false, 'draft');
    window.savedEditor = editor;
  });
  await memo.waitForFunction(() => document.getElementById('memo-editor').textContent.includes('draft'));
  await memo.evaluate(() => {
    const selection = getSelection();
    window.savedSelection = [selection.anchorNode, selection.anchorOffset, window.savedEditor.textContent];
  });
  await web.$eval('#web-url', input => { input.value = 'https://example.com/자동'; window.savedInput = input; });
  await manager.bringToFront();
  await manager.click('#settings-nav');
  await manager.click('#settings-language-nav');
  for (const locale of ['ja', 'ko', 'en', 'system']) {
    await manager.select('#app-language', locale);
    const resolved = locale === 'system' ? 'ja' : locale;
    await manager.waitForFunction(value => document.documentElement.lang === value && !document.getElementById('app-language').disabled, {}, resolved);
    for (const page of pages) assert.equal(await page.evaluate(() => document.documentElement.lang), resolved);
    if (resolved !== 'ko') {
      const leftover = await manager.evaluate(() => {
        const exempt = '#settings-language-nav, #language-heading, option[lang], script, style';
        const found = [];
        for (const el of document.querySelectorAll('body *, title')) {
          if (el.closest(exempt)) continue;
          const values = Array.from(el.childNodes).filter(n => n.nodeType === Node.TEXT_NODE).map(n => n.data);
          for (const attr of ['title','placeholder','aria-label','alt']) values.push(el.getAttribute(attr) || '');
          for (const text of values) if (/[가-힣]/.test(text)) found.push(text.trim());
        }
        return found;
      });
      assert.deepEqual(leftover, [], 'untranslated manager labels');
    }
    const labels = {ja:['設定','本文をコピー','ウェブアドレス'], ko:['설정','본문 복사','웹 주소'], en:['Settings','Copy text','Web address']}[resolved];
    assert.equal(await manager.$eval('#settings-nav', e => e.textContent), labels[0]);
    assert.equal(await memo.$eval('#copy', e => e.getAttribute('aria-label')), labels[1]);
    assert.equal(await web.$eval('#web-url', e => e.getAttribute('aria-label')), labels[2]);
    const editorState = await memo.evaluate(() => {
      const selection = getSelection();
      return {
        sameEditor: window.savedEditor === document.querySelector('#memo-editor'),
        sameNode: window.savedSelection[0] === selection.anchorNode,
        sameOffset: window.savedSelection[1] === selection.anchorOffset,
        sameText: window.savedSelection[2] === window.savedEditor.textContent,
        hasDraft: window.savedEditor.textContent.includes('draft'),
      };
    });
    assert.deepEqual(editorState, {sameEditor:true,sameNode:true,sameOffset:true,sameText:true,hasDraft:true});
    assert(await term.evaluate(() => terminal.buffer.active.getLine(0)?.translateToString(true) === '자동 업데이트' && Boolean(window.outputChannel)));
    assert(await web.$eval('#web-url', e => e === window.savedInput && e.value === 'https://example.com/자동'));
    assert(await manager.evaluate(() => {
      const select = document.getElementById('app-language').getBoundingClientRect();
      return document.documentElement.scrollWidth <= innerWidth && select.left >= 0 && select.right <= innerWidth;
    }));
  }
  await manager.evaluate(() => window.jajanLanguage.apply({choice:'en',locale:'en',revision:0}));
  assert.equal(await manager.evaluate(() => document.documentElement.lang), 'ja');
  await term.evaluate(() => window.outputChannel.onmessage([...new TextEncoder().encode(' OK')]));
  await term.waitForFunction(() => terminal.buffer.active.getLine(0)?.translateToString(true).endsWith(' OK'));
  await manager.evaluate(() => { window.failLanguageSave = true; });
  await manager.select('#app-language', 'ko');
  await manager.waitForFunction(() => !document.getElementById('language-error').hidden);
  assert.equal(await manager.$eval('#app-language', e => e.value), 'system');
  assert.equal(await manager.evaluate(() => document.documentElement.lang), 'ja');
  assert.deepEqual(errors, []);
  console.log('PASS: prepaint gate, four choices, four pages, failed save, stale reply, draft, cursor, xterm output channel, URL and minimum manager width');
} finally { await browser.close(); }
