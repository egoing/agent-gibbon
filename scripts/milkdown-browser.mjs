// Actual bundled editor and app; native IPC is simulated, native IME is separate.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
const { chromium } = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
const root = fileURLToPath(new URL('../ui/', import.meta.url));
const output = process.env.GIBBON_SMOKE_OUTPUT || '/tmp/gibbon174-browser';
fs.mkdirSync(output, { recursive: true });
const browser = await chromium.launch({ headless: true });
const errors = [], requests = [], results = [];
async function open(markdown, size = { width: 300, height: 240 }) {
  const page = await browser.newPage({ viewport: size });
  page.on('pageerror', e => errors.push(e.message));
  page.on('request', r => { if (!r.url().startsWith('http://gibbon.test/')) requests.push(r.url()); });
  await page.addInitScript(({ markdown }) => {
    window.testNote = { title: 'Milkdown 검증', markdown, html: '<p>원문</p>', revision: 1, color: '#dfe2dd' };
    window.testState = { visible: true, focused: false, folded: false, always_on_top: true };
    window.testCalls = []; window.testListeners = {}; window.testFailure = null;
    window.__TAURI__ = {
      window: { getCurrentWindow: () => ({ label: 'note-milkdown-browser', startResizeDragging: async () => {} }) },
      webviewWindow: { getCurrentWebviewWindow: () => ({ label: 'note-milkdown-browser', listen: async () => () => {} }) },
      clipboardManager: { writeText: async text => { window.testClipboard = text; } },
      event: { listen: async (name, fn) => { window.testListeners[name] = fn; return () => {}; } },
      core: { invoke: async (name, args) => {
        window.testCalls.push({ name, args });
        if (name === 'snapshot') return { ...window.testState, note: window.testNote };
        if (name === 'fit_note') return true;
        if (name === 'feedback_action') return window.testListeners['feedback-action']({ payload: {owner:'note-milkdown-browser',action:args.action} });
        if (name === 'poll_fold') return window.testState.folded;
        if (name === 'toggle_fold') return window.testState.folded = !window.testState.folded;
        if (name === 'poll_warp') return false;
        if (name === 'acknowledge_feedback') return true;
        if (name === 'save_note') {
          if (window.testSaveDelay) await new Promise(resolve => setTimeout(resolve, window.testSaveDelay));
          if (window.testFailure) throw new Error(window.testFailure);
          if (args.expectedRevision !== window.testNote.revision) throw new Error('메모가 외부에서 갱신되었습니다');
          window.testNote = { ...window.testNote, markdown: args.markdown, revision: window.testNote.revision + 1 };
          return window.testNote;
        }
      } },
    };
  }, { markdown });
  await page.route('http://gibbon.test/**', route => {
    const name = new URL(route.request().url()).pathname.slice(1);
    if (['telemetry.js', 'updater-guard.js', 'memo-focus.js'].includes(name)) return route.fulfill({ body: '', contentType: 'text/javascript' });
    const file = path.join(root, name);
    return route.fulfill({ body: fs.readFileSync(file), contentType: /\.m?js$/.test(name) ? 'text/javascript' : name.endsWith('.css') ? 'text/css' : 'text/html' });
  });
  await page.goto('http://gibbon.test/index.html');
  await page.waitForSelector('#memo-editor[aria-readonly]');
  return page;
}
const tick = page => page.waitForTimeout(450);
const save = page => page.keyboard.press('Control+s');
try {
  const original = '# 제목\n\n본문 한글\n\n- [ ] 할 일\n- [x] 완료\n\n| 가 | 나 |\n| --- | --- |\n| 1 | 2 |\n\n```js\n[ ] 코드\n```\n';
  const page = await open(original);
  assert.equal(await page.locator('#memo-editor').getAttribute('aria-readonly'), 'false');
  assert.equal(await page.locator('#edit').count(), 0);
  await page.screenshot({ path: path.join(output, 'clean-300.png') });
  assert.equal(await page.evaluate(() => window.gibbonUpdateBusy()), false);
  await page.locator('#memo-editor p').first().click();
  await save(page); await tick(page);
  assert.equal(await page.evaluate(() => window.testCalls.filter(c => c.name === 'save_note').length), 0);
  assert.equal(await page.evaluate(() => window.testNote.markdown), original);
  await page.keyboard.press('End'); await page.keyboard.insertText(' 수정');
  assert.equal(await page.evaluate(() => window.gibbonUpdateBusy()), true);
  assert.equal(await page.locator('#save').isDisabled(), false);
  await page.screenshot({ path: path.join(output, 'dirty-300.png') });
  await save(page); await tick(page);
  assert.match(await page.evaluate(() => window.testNote.markdown), /본문 한글 수정/);
  assert.equal(await page.evaluate(() => window.gibbonUpdateBusy()), false);
  assert.equal(await page.locator('#memo-editor').getAttribute('contenteditable'), 'true');
  await page.keyboard.press(process.platform === 'darwin' ? 'Meta+z' : 'Control+z');
  assert.equal(await page.evaluate(() => window.gibbonUpdateBusy()), true);
  await page.keyboard.press(process.platform === 'darwin' ? 'Meta+Shift+Z' : 'Control+Shift+Z');
  assert.equal(await page.evaluate(() => window.gibbonUpdateBusy()), false);
  await page.getByRole('checkbox').first().click();
  await save(page); await tick(page);
  assert.match(await page.evaluate(() => window.testNote.markdown), /[-*] \[x\] 할 일/);
  // Active source revision remains the edit baseline across external updates.
  await page.locator('#memo-editor p').first().click(); await page.keyboard.insertText('초안');
  const draft = await page.locator('#memo-editor').innerText();
  await page.evaluate(() => { window.testNote = { ...window.testNote, revision: window.testNote.revision + 1, markdown: '외부 갱신' }; });
  await tick(page); await save(page); await tick(page);
  assert.equal(await page.locator('#memo-editor').innerText(), draft);
  assert.match(await page.locator('#message').innerText(), /외부/);
  await page.evaluate(() => { window.testState.folded = true; }); await tick(page);
  assert.equal(await page.locator('#editor').isVisible(), false);
  await page.evaluate(() => { window.testState.folded = false; window.testState.visible = false; }); await tick(page);
  await page.evaluate(() => { window.testState.visible = true; }); await tick(page);
  assert.equal(await page.locator('#memo-editor').innerText(), draft);
  await page.getByRole('button', {name:'변경 취소',exact:true}).click(); await tick(page);
  assert.equal(await page.locator('#memo-editor').innerText(), '외부 갱신');
  // Toolbar blank area is the only text-note drag handle.
  const before = await page.evaluate(() => window.testCalls.filter(c => c.name === 'drag_note').length);
  await page.locator('#memo-editor').click();
  assert.equal(await page.evaluate(() => window.testCalls.filter(c => c.name === 'drag_note').length), before);
  await page.locator('#controls').dispatchEvent('mousedown', { button: 0 });
  assert.equal(await page.evaluate(() => window.testCalls.filter(c => c.name === 'drag_note').length), before + 1);
  // Composition guards save/cancel and protects remote refresh until composition ends.
  await page.locator('#memo-editor').dispatchEvent('compositionstart');
  await page.locator('#memo-editor').dispatchEvent('compositionupdate', { data: '한글' });
  await page.evaluate(() => { window.testNote = { ...window.testNote, revision: window.testNote.revision + 1, markdown: '조합 중 수신' }; });
  await tick(page);
  assert.equal(await page.locator('#memo-editor').innerText(), '외부 갱신');
  const count = await page.evaluate(() => window.testCalls.filter(c => c.name === 'save_note').length);
  const composingText = await page.locator('#memo-editor').textContent();
  await page.keyboard.press(process.platform === 'darwin' ? 'Meta+z' : 'Control+z');
  assert.equal(await page.locator('#memo-editor').textContent(), composingText);
  await save(page); await page.keyboard.press('Escape');
  assert.equal(await page.evaluate(() => window.testCalls.filter(c => c.name === 'save_note').length), count);
  await page.locator('#memo-editor').dispatchEvent('compositionend'); await tick(page);
  assert.equal(await page.locator('#memo-editor').innerText(), '조합 중 수신');
  await page.evaluate(() => {
    window.testNote = { ...window.testNote, revision: window.testNote.revision + 1, markdown: '유지\n\n새 내용\n\n- [x] 완료\n', html: '<p>유지</p><p>새 내용</p><ul><li>완료</li></ul>',
      change_html: '<p>유지</p><p><mark class="change-removed">삭제 내용</mark></p><p><mark class="change-added">새 내용</mark></p><ul><li><mark class="change-modified"><span class="task-checkbox">☑</span> </mark>완료</li></ul>' };
    window.testState.highlight_revision = window.testNote.revision;
  });
  await tick(page);
  assert.equal(await page.locator('#memo-editor .change-added').textContent(), '새 내용');
  assert.equal(await page.locator('#memo-editor .change-removed').textContent(), '삭제 내용');
  assert.equal(await page.locator('#memo-editor li.change-modified').count(), 1);
  await page.locator('#memo-editor p').first().click(); await page.keyboard.insertText('수정'); await save(page); await tick(page);
  assert(!/삭제 내용|change-added|change-removed/.test(await page.evaluate(() => window.testNote.markdown)));
  await page.screenshot({ path: path.join(output, 'editor-300.png') });
  results.push('direct editing, no-op source preservation, explicit save, undo/redo, task, conflict, fold/hide, toolbar drag, composition guard');
  await page.close();

  const format = await open('제목으로 바꿀 문장\n\n일반 본문\n');
  const selectText = async target => {
    await target.locator('#memo-editor p').first().click();
    await target.evaluate(() => {
      const text = document.querySelector('#memo-editor p').firstChild;
      const range = document.createRange(); range.setStart(text, 0); range.setEnd(text, text.textContent.length);
      const selection = getSelection(); selection.removeAllRanges(); selection.addRange(range);
    });
    await target.waitForSelector('#selection-format:not([hidden])');
  };
  await selectText(format);
  const selected = await format.evaluate(() => getSelection().toString());
  await format.locator('#selection-format button[aria-label="굵게"]').click();
  assert.equal(await format.locator('#memo-editor strong').textContent(), selected);
  assert.equal(await format.evaluate(() => getSelection().toString()), selected);
  await format.locator('#selection-format button[aria-label="제목 1"]').click();
  assert.equal(await format.locator('#memo-editor h1').textContent(), selected);
  await format.locator('#selection-format button[aria-label="본문"]').click();
  assert.equal(await format.locator('#memo-editor h1').count(), 0);
  await format.screenshot({ path: path.join(output, 'selection-format-300.png') });
  await save(format); await tick(format);
  assert.match(await format.evaluate(() => window.testNote.markdown), /\*\*제목으로 바꿀 문장\*\*/);
  assert.equal(await format.evaluate(() => window.testCalls.filter(c => c.name === 'drag_note').length), 0);
  await format.locator('#memo-editor').dispatchEvent('compositionstart');
  assert.equal(await format.locator('#selection-format').isVisible(), false);
  await format.locator('#memo-editor').dispatchEvent('compositionend');
  await format.close();
  const narrowFormat = await open('좁은 창 선택 메뉴\n', { width: 144, height: 120 });
  await selectText(narrowFormat);
  const menuBounds = await narrowFormat.locator('#selection-format').boundingBox();
  assert(menuBounds.x >= 0 && menuBounds.x + menuBounds.width <= 144 && menuBounds.y >= 0 && menuBounds.y + menuBounds.height <= 120);
  await narrowFormat.locator('#memo-editor').press('Alt+F10');
  assert.equal(await narrowFormat.evaluate(() => document.activeElement.closest('#selection-format')?.id), 'selection-format');
  await narrowFormat.keyboard.press('End');
  await narrowFormat.keyboard.press('Enter');
  assert.equal(await narrowFormat.locator('#selection-format').isVisible(), true);
  await narrowFormat.screenshot({ path: path.join(output, 'selection-format-144.png') });
  await narrowFormat.close();
  results.push('selection format toolbar bold/heading/paragraph, retained selection, save, IME hiding, narrow bounds and keyboard access');

  const automatic = await open('자동 저장 본문\n');
  assert.equal(await automatic.locator('#save').isVisible(), false);
  await automatic.locator('#memo-editor p').click(); await automatic.keyboard.press('End');
  await automatic.keyboard.insertText(' 추가');
  await automatic.waitForFunction(() => window.testNote.markdown.includes('추가') && !window.gibbonUpdateBusy());
  assert.equal(await automatic.evaluate(() => window.testCalls.filter(c => c.name === 'save_note').length), 1);
  // A deliberately slow IPC write must not block or acknowledge newer typing.
  await automatic.evaluate(() => { window.testSaveDelay = 700; });
  await automatic.keyboard.insertText(' A');
  await automatic.waitForFunction(() => window.testCalls.filter(c => c.name === 'save_note').length === 2);
  assert.equal(await automatic.locator('#memo-editor').getAttribute('contenteditable'), 'true');
  await automatic.keyboard.insertText(' B');
  await automatic.waitForFunction(() => window.testNote.markdown.includes('A B') && !window.gibbonUpdateBusy());
  assert.equal(await automatic.evaluate(() => window.testCalls.filter(c => c.name === 'save_note').length), 3);
  await automatic.evaluate(() => { window.testSaveDelay = 0; window.testFailure = '원본 파일 외부 변경'; });
  await automatic.keyboard.insertText(' 충돌 초안');
  await automatic.waitForSelector('#actions button[data-action="save"]');
  const failedCount = await automatic.evaluate(() => window.testCalls.filter(c => c.name === 'save_note').length);
  await automatic.keyboard.insertText(' 추가 초안'); await automatic.waitForTimeout(900);
  assert.equal(await automatic.evaluate(() => window.testCalls.filter(c => c.name === 'save_note').length), failedCount);
  assert.match(await automatic.locator('#memo-editor').textContent(), /충돌 초안 추가 초안/);
  await automatic.evaluate(() => { window.testFailure = null; });
  await automatic.getByRole('button', {name:'저장 ·',exact:false}).click();
  await automatic.waitForFunction(() => window.testNote.markdown.includes('추가 초안') && !window.gibbonUpdateBusy());
  await automatic.screenshot({path:path.join(output,'autosave-300.png')});
  await automatic.close();
  const imeAutomatic = await open('조합 자동 저장\n');
  await imeAutomatic.locator('#memo-editor p').click();
  await imeAutomatic.locator('#memo-editor').dispatchEvent('compositionstart');
  await imeAutomatic.keyboard.insertText('한글');
  await imeAutomatic.waitForTimeout(850);
  assert.equal(await imeAutomatic.evaluate(() => window.testCalls.filter(c => c.name === 'save_note').length), 0);
  await imeAutomatic.locator('#memo-editor').dispatchEvent('compositionend');
  await imeAutomatic.waitForFunction(() => window.testNote.markdown.includes('한글') && !window.gibbonUpdateBusy());
  await imeAutomatic.close();
  results.push('600ms autosave, no save button, concurrent input during slow save, next revision, paused retry after failure, composition deferral');

  const hostile = '<script>window.HACKED=true</script>\n\n<a href="file:///etc/passwd">파일</a>\n\n![숨김](https://forbidden.test/image.png)\n\n[링크](https://forbidden.test/)';
  const security = await open(hostile);
  assert.equal(await security.locator('#editor img[src], #editor a[href], #editor script').count(), 0);
  assert.equal(await security.evaluate(() => window.HACKED), undefined);
  await security.evaluate(() => {
    const data = new DataTransfer(); data.setData('text/html', '<img src="https://forbidden.test/paste.png" onerror="window.HACKED=true">'); data.setData('text/plain', '붙여넣기 문자');
    document.querySelector('#memo-editor').dispatchEvent(new ClipboardEvent('paste', { bubbles: true, cancelable: true, clipboardData: data }));
  });
  await tick(security);
  assert.equal(await security.locator('#editor img[src], #editor a[href], #editor script').count(), 0);
  assert.match(await security.locator('#memo-editor').textContent(), /붙여넣기 문자/);
  assert.equal(await security.evaluate(() => window.HACKED), undefined);
  results.push('raw HTML inert, image/link no request/navigation, HTML paste plain-text only');
  await security.close();

  // Any unsupported roundtrip is shown as literal source and never editable/saved.
  const unsupported = '[본문][ref]\n\n[ref]: https://example.test/ "참조"\n';
  const protectedPage = await open(unsupported);
  if (await protectedPage.locator('#memo-editor').getAttribute('aria-readonly') === 'true') {
    assert.equal(await protectedPage.locator('#memo-editor code').textContent(), unsupported);
    await save(protectedPage); await tick(protectedPage);
    assert.equal(await protectedPage.evaluate(() => window.testCalls.filter(c => c.name === 'save_note').length), 0);
    await protectedPage.locator('#copy').evaluate(button => button.click());
    assert.equal(await protectedPage.evaluate(() => window.testClipboard), unsupported);
    results.push('unsupported reference syntax protected as literal original and copied losslessly');
  } else results.push('reference syntax preserved by upstream roundtrip');
  await protectedPage.close();

  for (const size of [{ width: 144, height: 96 }, { width: 300, height: 210 }, { width: 480, height: 360 }]) {
    const small = await open('# 제목\n\n- [ ] 항목\n\n본문\n'.repeat(10), size);
    await small.locator('#memo-editor').click(); await small.keyboard.insertText('추가');
    await tick(small);
    const bounds = await small.locator('#feedback').boundingBox();
    assert(bounds && bounds.x >= 0 && bounds.y >= 0 && bounds.x + bounds.width <= size.width && bounds.y + bounds.height <= size.height);
    await small.screenshot({ path: path.join(output, `editor-${size.width}x${size.height}.png`) });
    await small.close();
  }
  results.push('144/300/480px layout and actions');
  const large = await open('문단 한글 데이터\n\n'.repeat(6000), { width: 480, height: 360 });
  assert.equal(await large.locator('#memo-editor').getAttribute('aria-readonly'), 'false');
  await large.locator('#memo-editor p').first().click(); await large.keyboard.insertText('편집');
  await save(large); await tick(large);
  assert.match(await large.evaluate(() => window.testNote.markdown), /편집/);
  results.push('large 66k-character document edit/save');
  await large.close();
  assert.deepEqual(errors, []); assert.deepEqual(requests, []);
  console.log(JSON.stringify({ browser: browser.version(), nativeIPC: 'simulated', nativeIME: 'not tested', results, output, errors, externalRequests: requests }, null, 2));
} finally { await browser.close(); }
