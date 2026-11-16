// Actual bundled editor, app and Feedback DOM. Native IPC is simulated; native IME is separate.
// Build first: npm run build:editor. Optional PLAYWRIGHT_MODULE and PLAYWRIGHT_CHROMIUM_EXECUTABLE locate external browser tooling.
// GIBBON_BASELINE=1 is intentionally only for the pre-fix implementation; it proves cancel destroys native editor undo history.
// Regression mode uses real editor input, real Feedback clicks and explicit stale native-action events.
import fs from 'node:fs';
import { build } from 'esbuild';
import path from 'node:path';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
const { chromium } = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
const root = fileURLToPath(new URL('../ui/', import.meta.url));
const output = process.env.GIBBON_SMOKE_OUTPUT || '/tmp/gibbon15-cancel-browser';
const stamp = () => ({root:process.cwd(),time:new Date().toISOString(),kst:new Intl.DateTimeFormat('sv-SE',{timeZone:'Asia/Seoul',dateStyle:'short',timeStyle:'medium'}).format(new Date())+' KST'});
fs.mkdirSync(output, { recursive: true });
const browser = await chromium.launch({ headless: true, ...(process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE ? { executablePath: process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE } : {}) });
const errors = [], requests = [], results = [];
// These two scenarios need a real plugin transaction with no DOM gesture.
// The harness exposes the bundled dependency's view only inside its test page.
let probeBundle;
async function historyProbeBundle() {
  if (!probeBundle) probeBundle = build({
    stdin: {
      resolveDir: path.dirname(path.resolve(root)),
      contents: `
        export { createMemoEditor } from './ui/milkdown-editor.js';
        import { EditorView } from '@milkdown/prose/view';
        import { Plugin } from '@milkdown/prose/state';
        import { redoDepth, redo } from '@milkdown/prose/history';
        const dispatch = EditorView.prototype.dispatch;
        EditorView.prototype.dispatch = function (tr) {
          window.historyProbe = { view: this, Plugin, redoDepth, redo };
          return dispatch.call(this, tr);
        };
      `,
    },
    bundle: true, format: 'esm', platform: 'browser', target: 'es2020', write: false,
  });
  return (await probeBundle).outputFiles[0].text;
}
async function open(markdown, size = { width: 300, height: 240 }, probeHistory = false) {
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
        if (name === 'export_note') { if (window.testExportDelay) await new Promise(resolve=>setTimeout(resolve,window.testExportDelay));return null; }
        if (name === 'feedback_action') return window.testListeners['feedback-action']({ payload: {owner:'note-milkdown-browser',action:args.action,revision:args.revision} });
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
  await page.route('http://gibbon.test/**', async route => {
    const name = new URL(route.request().url()).pathname.slice(1);
    if (['telemetry.js', 'updater-guard.js', 'memo-focus.js'].includes(name)) return route.fulfill({ body: '', contentType: 'text/javascript' });
    if (probeHistory && name === 'generated/milkdown.js') {
      return route.fulfill({ body: await historyProbeBundle(), contentType: 'text/javascript' });
    }
    const file = path.join(root, name);
    return route.fulfill({ body: fs.readFileSync(file), contentType: /\.m?js$/.test(name) ? 'text/javascript' : name.endsWith('.css') ? 'text/css' : 'text/html' });
  });
  await page.goto('http://gibbon.test/index.html');
  await page.waitForSelector('#memo-editor[aria-readonly]');
  return page;
}
const tick = page => page.waitForTimeout(450);
const save = page => page.keyboard.press('Control+s');
const undo = page => page.keyboard.press(process.platform === 'darwin' ? 'Meta+z' : 'Control+z');
const text = page => page.locator('#memo-editor').innerText();
const busy = page => page.evaluate(() => window.gibbonUpdateBusy());
const token = page => page.evaluate(() => window.testCalls.filter(c=>c.name==='present_feedback').at(-1)?.args.view.buttons.find(button=>button.id==='restore_draft')?.revision ?? window.testCalls.filter(c=>c.name==='present_feedback').at(-1)?.args.view.revision);
async function dispatchRestore(page,revision) { await page.evaluate(revision=>window.testListeners['feedback-action']({payload:{owner:'note-milkdown-browser',action:'restore_draft',revision}}),revision); }
const restoreButton = page => page.getByRole('button', {name:'초안 복구',exact:true});
async function draft(page, suffix = ' 초안') {
 await page.evaluate(() => { window.testFailure = '검증용 저장 실패'; });
 await page.locator('#memo-editor p').first().click(); await page.keyboard.press('End'); await page.keyboard.insertText(suffix);
 await page.waitForSelector('#actions button[data-action="cancel"]');
 return text(page);
}
async function cancel(page) { await page.getByRole('button',{name:'변경 취소',exact:true}).click(); await tick(page); }
async function restore(page) {
 await restoreButton(page).click();
 assert.equal(await page.evaluate(() => window.testCalls.filter(c => c.name === 'feedback_action').at(-1).args.action), 'restore_draft');
}
async function remote(page, markdown, color) {
 await page.evaluate(({markdown,color}) => { window.testNote={...window.testNote,revision:window.testNote.revision+1,...(markdown === null ? {}:{markdown}),...(color ? {color}:{})}; },{markdown,color});
 await tick(page);
}
async function scenario(name, fn, size, probeHistory = false) {
 const page=await open('원문\n',size,probeHistory);
 try { await fn(page); results.push(name);if(name.startsWith('integration '))fs.writeFileSync(path.join(output,`integration-dom-${results.length}.json`),JSON.stringify({name,trace:await page.evaluate(()=>window.integrationTrace)},null,2));console.log('PASS',name); }
 catch(error) { await page.screenshot({path:path.join(output,`failure-${results.length}.png`)}); fs.writeFileSync(path.join(output,'failure.json'),JSON.stringify({...stamp(),name,completed:results,error:error.stack,state:await page.evaluate(()=>({note:window.testNote,calls:window.testCalls,body:document.body.innerText,activity:window.gibbonUpdateActivity()}))},null,2));throw new Error(`${name}: ${error.stack}`); }
 finally { await page.close(); }
}
// Integration-only additions: invoke the actual selection menu, never synthetic schema changes.
async function selectFirstBlock(page) {
 await page.locator('#memo-editor').locator('p,h1,h2').first().click();
 await page.evaluate(()=>{
  const block=document.querySelector('#memo-editor p, #memo-editor h1, #memo-editor h2');
  const range=document.createRange();range.selectNodeContents(block);
  const selection=getSelection();selection.removeAllRanges();selection.addRange(range);
 });
 await page.waitForSelector('#selection-format:not([hidden])');
}
async function formatSelection(page,label) {
 await selectFirstBlock(page);
 const selected=await page.evaluate(()=>getSelection().toString());
 const target=page.locator('#selection-format').getByRole('button',{name:label,exact:true});
 await page.evaluate(label=>{(window.integrationTrace ||= []).push({stage:'before-menu-click',label,html:document.querySelector('#memo-editor').innerHTML,selected:getSelection().toString(),target:[...document.querySelectorAll('#selection-format button')].find(b=>b.getAttribute('aria-label')===label)?.outerHTML});},label);
 await target.click();
 await page.evaluate(label=>window.integrationTrace.push({stage:'after-menu-click',label,html:document.querySelector('#memo-editor').innerHTML,selected:getSelection().toString()}),label);
 assert.equal(await page.evaluate(()=>getSelection().toString()),selected);
}
async function formatCheckpoint(page,stage,code=false) {
 assert.equal(await page.locator('#memo-editor strong').count(),0);assert.equal(await page.locator('#memo-editor code').count(),code?1:0);
 await page.evaluate(stage=>{(window.integrationTrace ||= []).push({stage,html:document.querySelector('#memo-editor').innerHTML,selected:getSelection().toString()});},stage);
}
const redoKeyboard=page=>page.keyboard.press(process.platform==='darwin'?'Meta+Shift+z':'Control+Shift+z');
async function formattedCancel(page,label) {
 await page.evaluate(()=>{window.testFailure='검증용 저장 실패';});
 await formatSelection(page,label);await save(page);
 await page.waitForSelector('#actions button[data-action="cancel"]');
}
try {
 if(process.env.GIBBON_BASELINE === '1') {
  await scenario('baseline cancel loses undo draft',async page=>{
   const before=await draft(page,' 잃어버릴 초안'); await cancel(page); const cancelled=await text(page);
   await page.locator('#memo-editor p').click(); await undo(page);
   const evidence={...stamp(),before,cancelled,afterUndo:await text(page),restoreButtons:await restoreButton(page).count()};
   fs.writeFileSync(path.join(output,'baseline.json'),JSON.stringify(evidence,null,2));
   assert.equal(evidence.afterUndo,'원문');assert.equal(evidence.restoreButtons,0);
  });
 } else {
  await scenario('cancel restore real feedback action, dirty, undo, save CAS',async page=>{
   const before=await draft(page); await cancel(page); assert.equal(await text(page),'원문'); assert.equal(await busy(page),false);
   assert.equal(await restoreButton(page).isVisible(),true); await restore(page); assert.equal(await text(page),before);assert.equal(await busy(page),true);
   await page.locator('#memo-editor p').click(); await undo(page); assert.equal(await text(page),'원문');
   await page.keyboard.press(process.platform === 'darwin' ? 'Meta+Shift+z':'Control+Shift+z');assert.equal(await text(page),before);
   await page.evaluate(()=>{window.testFailure=null;}); await save(page);await tick(page);
   assert.match(await page.evaluate(()=>window.testNote.markdown),/초안/);assert.equal(await busy(page),false);
   assert.equal(await page.evaluate(()=>window.testCalls.filter(c=>c.name==='save_note').at(-1).args.expectedRevision),1);
  });
  await scenario('same revision resumes normal autosave after explicit restore', async page => {
    const before = await draft(page);
    await cancel(page);
    await page.evaluate(() => { window.testFailure = null; });
    await restore(page);
    await page.waitForFunction(() => window.testNote.revision === 2 && !window.gibbonUpdateBusy());
    assert.equal((await page.evaluate(() => window.testNote.markdown)).trimEnd(), before);
    assert.equal(await page.evaluate(() => window.testCalls.filter(c => c.name === 'save_note').length), 2);
  });
  for(const timing of ['before','after']) await scenario(`external update ${timing} cancel retains baseline CAS`,async page=>{
   const before=await draft(page); if(timing==='before')await remote(page,'외부 본문\n');await cancel(page);
   if(timing==='after')await remote(page,'외부 본문\n');await restore(page);assert.equal(await text(page),before);
   const pending=await page.evaluate(()=>window.testCalls.filter(c=>c.name==='save_note').length);await page.waitForTimeout(750);assert.equal(await page.evaluate(()=>window.testCalls.filter(c=>c.name==='save_note').length),pending);
   await page.evaluate(()=>{window.testFailure=null;});await save(page);await tick(page);
   assert.equal(await page.evaluate(()=>window.testNote.markdown),'외부 본문\n');assert.equal(await busy(page),true);
   assert.equal(await page.evaluate(()=>window.testCalls.filter(c=>c.name==='save_note').at(-1).args.expectedRevision),1);
  });
  await scenario('color revision cannot silently rebase recovered draft',async page=>{
   const before=await draft(page);await cancel(page);await remote(page,null,'#ffeedd');await restore(page);
   assert.equal(await text(page),before);await page.evaluate(()=>{window.testFailure=null;});await save(page);await tick(page);
   assert.equal(await page.evaluate(()=>window.testCalls.filter(c=>c.name==='save_note').at(-1).args.expectedRevision),1);
   assert.equal(await busy(page),true);
  });
  await scenario('same-value remote restore consumes candidate clean without save',async page=>{
   const before=await draft(page);await cancel(page);await remote(page,before+'\n');
   const count=await page.evaluate(()=>window.testCalls.filter(c=>c.name==='save_note').length);await restore(page);
   assert.equal(await text(page),before);assert.equal(await busy(page),false);assert.equal(await restoreButton(page).count(),0);
   await save(page);await tick(page);assert.equal(await page.evaluate(()=>window.testCalls.filter(c=>c.name==='save_note').length),count);
  });
  await scenario('restore failure retry preserves text and blocked state',async page=>{
   const before=await draft(page);await cancel(page);await restore(page);await save(page);await tick(page);
   assert.equal(await text(page),before);assert.equal(await busy(page),true);assert.match(await page.locator('#message').innerText(),/실패/);
  });
  await scenario('consecutive cancel keeps most recent draft and clean cancel does not destroy it',async page=>{
   const before=await draft(page);await cancel(page);await restore(page);await save(page);await tick(page);await cancel(page);
   await page.evaluate(()=>window.testListeners['feedback-action']({payload:{owner:'note-milkdown-browser',action:'cancel'}}));
   await restore(page);assert.equal(await text(page),before);
  });
  await scenario('hide preserves recovery; Escape hides without cancel',async page=>{
   const before=await draft(page);await cancel(page);await page.evaluate(()=>{window.testState.visible=false;});await tick(page);
   await page.evaluate(()=>{window.testState.visible=true;});await tick(page);await restore(page);assert.equal(await text(page),before);
   await page.keyboard.press('Escape');await tick(page);assert.equal(await text(page),before);
   assert.equal(await page.evaluate(()=>window.testCalls.some(c=>c.name==='hide_note')),false);
  });
  await scenario('composition prevents restore and cancel until compositionend',async page=>{
   const before=await draft(page);await cancel(page);const revision=await token(page);await page.locator('#memo-editor').dispatchEvent('compositionstart');
   await dispatchRestore(page,revision);
   assert.equal(await text(page),'원문');await page.locator('#memo-editor').dispatchEvent('compositionend');await restore(page);assert.equal(await text(page),before);
  });
  await scenario('new typing never overwritten by stale restore action',async page=>{
   await draft(page);await cancel(page);const revision=await token(page);await page.locator('#memo-editor p').click();await page.keyboard.press('End');await page.keyboard.insertText(' 새로운 입력');
   const before=await text(page);await dispatchRestore(page,revision);
   assert.equal(await text(page),before);
  });
  await scenario('undo newer input to clean never revives consumed candidate',async page=>{
   await draft(page);await cancel(page);const revision=await token(page);
   await page.locator('#memo-editor p').click();await page.keyboard.press('End');await page.keyboard.insertText(' 새 입력');await undo(page);
   assert.equal(await busy(page),false);assert.equal(await restoreButton(page).count(),0);await dispatchRestore(page,revision);assert.equal(await text(page),'원문');
  });
  await scenario('old generation cannot restore newer cancellation',async page=>{
   await draft(page,' 첫 초안');await cancel(page);const revision=await token(page);
   const next=await draft(page,' 둘째 초안');await cancel(page);await dispatchRestore(page,revision);
   assert.equal(await text(page),'원문');assert.equal(await restoreButton(page).isVisible(),true);await restore(page);assert.equal(await text(page),next);
  });
  await scenario('restored stale redo retains old CAS then undo fresh input uses latest CAS',async page=>{
   await draft(page);await cancel(page);await remote(page,'최신 원문\n');await restore(page);
   await page.locator('#memo-editor p').click();await undo(page);assert.equal(await text(page),'최신 원문');
   await page.keyboard.press(process.platform === 'darwin' ? 'Meta+Shift+z':'Control+Shift+z');
   const count=await page.evaluate(()=>window.testCalls.filter(c=>c.name==='save_note').length);await page.waitForTimeout(750);assert.equal(await page.evaluate(()=>window.testCalls.filter(c=>c.name==='save_note').length),count);
   await page.evaluate(()=>{window.testFailure=null;});await save(page);await tick(page);assert.equal(await page.evaluate(()=>window.testCalls.filter(c=>c.name==='save_note').at(-1).args.expectedRevision),1);
   await page.locator('#memo-editor p').click();await undo(page);await page.keyboard.press('End');await page.keyboard.insertText(' 새 가지');await save(page);await tick(page);
   assert.equal(await page.evaluate(()=>window.testCalls.filter(c=>c.name==='save_note').at(-1).args.expectedRevision),2);assert.match(await page.evaluate(()=>window.testNote.markdown),/최신 원문 새 가지/);
  });
  for (const mode of ['direct', 'appended']) {
    await scenario(`redo after ${mode} non-history change keeps original save revision`, async page => {
      const before = await draft(page);
      await cancel(page);
      await remote(page, '최신 원문\n');
      await restore(page);
      await page.locator('#memo-editor p').click();
      await undo(page);
      assert.equal(await text(page), '최신 원문');
      const depth = await page.evaluate(mode => {
        const { view, Plugin, redoDepth } = window.historyProbe;
        const insert = state => state.tr.insertText('!', 2);
        const initial = redoDepth(view.state);
        if (mode === 'direct') {
          view.dispatch(insert(view.state).setMeta('addToHistory', false));
        } else {
          const append = new Plugin({
            appendTransaction(transactions, _, state) {
              if (transactions.some(tr => tr.getMeta('probeAppend'))) return insert(state);
            },
          });
          view.updateState(view.state.reconfigure({ plugins: [...view.state.plugins, append] }));
          view.dispatch(view.state.tr.setMeta('probeAppend', true).setMeta('addToHistory', false));
        }
        return [initial, redoDepth(view.state)];
      }, mode);
      assert.deepEqual(depth, [1, 1]);
      assert.equal(await page.evaluate(() => {
        const { view, redo } = window.historyProbe;
        return redo(view.state, view.dispatch);
      }), true);
      assert.equal(await text(page), before);
      const writes = () => page.evaluate(() => window.testCalls.filter(c => c.name === 'save_note'));
      const initialWrites = (await writes()).length;
      await page.evaluate(() => { window.testFailure = null; });
      await page.waitForTimeout(750);
      assert.equal((await writes()).length, initialWrites);
      await save(page);
      await tick(page);
      assert.equal((await writes()).at(-1).args.expectedRevision, 1);
      assert.equal(await page.evaluate(() => window.testNote.markdown), '최신 원문\n');
    }, undefined, true);
  }
  await scenario('typing latest text clean preserves stale undo ancestry CAS',async page=>{
   await draft(page);await cancel(page);await remote(page,'최신 원문\n');await restore(page);
   await page.locator('#memo-editor p').click();await page.keyboard.press(process.platform === 'darwin' ? 'Meta+a':'Control+a');await page.keyboard.insertText('최신 원문');
   assert.equal(await text(page),'최신 원문');assert.equal(await busy(page),false);
   await page.keyboard.insertText(' 후속');const count=await page.evaluate(()=>window.testCalls.filter(c=>c.name==='save_note').length);await page.waitForTimeout(750);
   assert.equal(await page.evaluate(()=>window.testCalls.filter(c=>c.name==='save_note').length),count);
   await page.evaluate(()=>{window.testFailure=null;});await save(page);await tick(page);
   assert.equal(await page.evaluate(()=>window.testCalls.filter(c=>c.name==='save_note').at(-1).args.expectedRevision),1);assert.equal(await page.evaluate(()=>window.testNote.markdown),'최신 원문\n');
  });
  await scenario('protected source rejects restore without consuming recovery',async page=>{
   const before=await draft(page);await cancel(page);const revision=await token(page);
   await remote(page,'[본문][ref]\n\n[ref]: https://example.test/ "참조"\n');
   assert.equal(await page.locator('#memo-editor').getAttribute('aria-readonly'),'true');await dispatchRestore(page,revision);
   assert.equal(await page.locator('#memo-editor code').textContent(),'[본문][ref]\n\n[ref]: https://example.test/ "참조"\n');
   await remote(page,'복구 가능한 최신 원문\n');await restore(page);assert.equal(await text(page),before);
  });
  await scenario('exporting rejects delayed recovery action and retains candidate',async page=>{
   const before=await draft(page);await cancel(page);const revision=await token(page);await page.evaluate(()=>{window.testExportDelay=900;});
   await page.evaluate(()=>{window.testListeners['feedback-action']({payload:{owner:'note-milkdown-browser',action:'export'}});});
   await page.waitForFunction(()=>window.gibbonUpdateActivity().exporting);await dispatchRestore(page,revision);assert.equal(await text(page),'원문');
   await page.waitForFunction(()=>!window.gibbonUpdateActivity().exporting);await restore(page);assert.equal(await text(page),before);
  });
  await scenario('Escape saves and hides without creating recovery',async page=>{
   await page.locator('#memo-editor p').click();await page.keyboard.press('End');await page.keyboard.insertText(' Escape 저장');await page.keyboard.press('Escape');
   await page.waitForFunction(()=>window.testCalls.some(c=>c.name==='hide_note'));
   assert.match(await page.evaluate(()=>window.testNote.markdown),/Escape 저장/);assert.equal(await restoreButton(page).count(),0);
  });
  await scenario('144px real feedback keyboard Tab Enter recovery',async page=>{
   const before=await draft(page);await cancel(page);await page.locator('#memo-editor p').click();
   await page.keyboard.press('Tab');
   assert.equal(await page.evaluate(()=>document.activeElement?.dataset.action),'restore_draft');
   const bounds=await restoreButton(page).boundingBox();assert(bounds.x>=0 && bounds.x+bounds.width<=144);
   await page.screenshot({path:path.join(output,'recovery-feedback-144.png')});
   await page.keyboard.press('Enter');assert.equal(await text(page),before);assert.equal(await busy(page),true);
  },{width:144,height:240});
  await scenario('independent WebViews never share draft candidates',async page=>{
   const first=await draft(page,' 첫 메모 초안');await cancel(page);
   const other=await open('둘째 메모\n');
   try {
    assert.equal(await restoreButton(other).count(),0);const second=await draft(other,' 둘째 메모 초안');await cancel(other);
    await restore(page);assert.equal(await text(page),first);assert.equal(await text(other),'둘째 메모');assert.equal(await restoreButton(other).isVisible(),true);
    await restore(other);assert.equal(await text(other),second);assert.equal(await text(page),first);
   } finally {await other.close();}
  });
  await scenario('saving ignores stale restore without overwriting newer typing',async page=>{
   await draft(page);await cancel(page);const revision=await token(page);await page.evaluate(()=>{window.testFailure=null;window.testSaveDelay=900;});
   await page.locator('#memo-editor p').click();await page.keyboard.press('End');await page.keyboard.insertText(' 새 저장');
   await save(page);await page.waitForFunction(()=>window.gibbonUpdateActivity().saving);
   await page.keyboard.insertText(' 후속 입력');const before=await text(page);
   await dispatchRestore(page,revision);
   assert.equal(await text(page),before);await page.waitForFunction(()=>!window.gibbonUpdateBusy());assert.match(await page.evaluate(()=>window.testNote.markdown),/후속 입력/);
  });
  // #183 + #15 integration scenarios; all 23 original regression cases above are retained.
  const formatCases=[
   {label:'큰제목',selector:'h1',next:'작은제목',serialized:'## 원문\n'},
   {label:'작은제목',selector:'h2',next:'코드',serialized:'## `원문`\n'},
   {label:'코드',selector:'p code',next:'큰제목',serialized:'# `원문`\n'},
  ];
  for(const item of formatCases) {
   await scenario(`integration ${item.label} recovery undo redo actual menu normal autosave`,async page=>{
    await formattedCancel(page,item.label);await formatCheckpoint(page,'formatted-before-cancel',item.label==='코드');assert.equal(await page.locator('#memo-editor '+item.selector).textContent(),'원문');
    await cancel(page);assert.equal(await page.locator('#memo-editor p').textContent(),'원문');await formatCheckpoint(page,'cancelled-clean');await restore(page);await formatCheckpoint(page,'restored',item.label==='코드');
    assert.equal(await page.locator('#memo-editor '+item.selector).textContent(),'원문');
    await undo(page);await formatCheckpoint(page,'undo-clean');assert.equal(await page.locator('#memo-editor p').textContent(),'원문');assert.equal(await busy(page),false);
    await redoKeyboard(page);await formatCheckpoint(page,'redo-restored',item.label==='코드');assert.equal(await page.locator('#memo-editor '+item.selector).textContent(),'원문');
    await page.evaluate(()=>{window.testFailure=null;});await formatSelection(page,item.next);await formatCheckpoint(page,'after-next-menu',item.label==='코드'||item.next==='코드');
    await page.waitForFunction(()=>window.testNote.revision===2 && !window.gibbonUpdateBusy());
    const written=await page.evaluate(()=>window.testCalls.filter(c=>c.name==='save_note').at(-1));
    assert.equal(written.args.expectedRevision,1);assert.equal(written.args.markdown,item.serialized);
    assert.equal(await restoreButton(page).count(),0);
   });
   await scenario(`integration ${item.label} stale recovery menu keeps external CAS and blocks autosave`,async page=>{
    await formattedCancel(page,item.label);await formatCheckpoint(page,'formatted-before-cancel',item.label==='코드');await cancel(page);await remote(page,'외부 최신\n');await restore(page);await formatCheckpoint(page,'stale-restored',item.label==='코드');
    await undo(page);await formatCheckpoint(page,'stale-undo-clean');assert.equal(await text(page),'외부 최신');await redoKeyboard(page);await formatCheckpoint(page,'stale-redo',item.label==='코드');
    assert.equal(await page.locator('#memo-editor '+item.selector).textContent(),'원문');
    await formatSelection(page,item.next);await formatCheckpoint(page,'stale-after-next-menu',item.label==='코드'||item.next==='코드');const count=await page.evaluate(()=>window.testCalls.filter(c=>c.name==='save_note').length);
    await page.waitForTimeout(750);assert.equal(await page.evaluate(()=>window.testCalls.filter(c=>c.name==='save_note').length),count);
    await page.evaluate(()=>{window.testFailure=null;});await save(page);await tick(page);
    assert.equal(await page.evaluate(()=>window.testCalls.filter(c=>c.name==='save_note').at(-1).args.expectedRevision),1);
    assert.equal(await page.evaluate(()=>window.testNote.markdown),'외부 최신\n');assert.equal(await busy(page),true);
   });
  }
  await scenario('integration 144px heading code keyboard selection menu recovery',async page=>{
   await formattedCancel(page,'큰제목');await cancel(page);await restore(page);await selectFirstBlock(page);
   await page.locator('#memo-editor').press('Alt+F10');await page.keyboard.press('End');
   await page.keyboard.press('ArrowLeft'); // Small heading in the unchanged action order.
   assert.equal(await page.evaluate(()=>document.activeElement.getAttribute('aria-label')),'작은제목');await page.keyboard.press('Enter');
   assert.equal(await page.locator('#memo-editor h2').textContent(),'원문');
   await selectFirstBlock(page);await page.locator('#memo-editor').press('Alt+F10');await page.keyboard.press('Home');
   await page.keyboard.press('ArrowRight');await page.keyboard.press('ArrowRight');await page.keyboard.press('ArrowRight');
   assert.equal(await page.evaluate(()=>document.activeElement.getAttribute('aria-label')),'코드');await page.keyboard.press('Space');
   assert.equal(await page.locator('#memo-editor h2 code').textContent(),'원문');
   const menu=await page.locator('#selection-format').boundingBox();assert(menu.x>=0 && menu.x+menu.width<=144);
   assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=144),true);
   await page.screenshot({path:path.join(output,'integration-format-recovery-144.png')});
   await save(page);await page.waitForSelector('#actions button[data-action="cancel"]');await cancel(page);await restore(page);
   assert.equal(await page.locator('#memo-editor h2 code').textContent(),'원문');
  },{width:144,height:240});

 }
 assert.deepEqual(errors,[]);assert.deepEqual(requests,[]);
 fs.writeFileSync(path.join(output,'results.json'),JSON.stringify({...stamp(),results,errors,requests},null,2));
} finally { await browser.close(); }
