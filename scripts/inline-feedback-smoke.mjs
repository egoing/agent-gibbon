// Browser layout and action regression for the single-window feedback footer.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
const {chromium}=await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
const root=fileURLToPath(new URL('..',import.meta.url));
const output=process.env.GIBBON_SMOKE_OUTPUT || '/tmp/gibbon7-inline-browser';
fs.mkdirSync(output,{recursive:true});
const browser=await chromium.launch({headless:true});
const errors=[];
const results=[];
try {
 const page=await browser.newPage();
 page.on('pageerror',error=>errors.push(error.message));
 await page.addInitScript(()=>{
  const listeners={};
  window.testCalls=[];
  window.testNote={revision:1,title:'내부 피드백 검증',markdown:'초안 원문',html:'<p>초안 원문</p>',color:'#dfe2dd'};
  window.__TAURI__={event:{listen:async(name,callback)=>{listeners[name]=callback;return()=>{};}},
   webviewWindow:{getCurrentWebviewWindow:()=>({label:'main',listen:async()=>()=>{}})},
   window:{getCurrentWindow:()=>({label:'main',startResizeDragging:async direction=>window.testCalls.push({name:'native-resize',direction})})},
   clipboardManager:{writeText:async()=>{}},core:{invoke:async(name,args)=>{
    window.testCalls.push({name,args});
    if(name==='snapshot')return {note:window.testNote,visible:true,focused:true,always_on_top:true};
    if(name==='memo_frame_focus')return true;
    if(name==='poll_fold')return !!window.testFolded;
    if(name==='toggle_fold'){window.testFolded=!window.testFolded;return window.testFolded;}
    if(name==='fit_note')return true;
    if(name==='save_note' && window.testSaveError)throw new Error('원본 파일이 변경되었습니다. '.repeat(25));
    if(name==='save_note'){window.testNote={...window.testNote,markdown:args.markdown,revision:window.testNote.revision+1};return window.testNote;}
    if(name==='feedback_action')listeners['feedback-action']?.({payload:{owner:'main',...args}});
    if(name==='export_note' && window.testExportError)throw new Error('원본 파일을 내보낼 수 없습니다. '.repeat(25));
    return null;
   }}};
 });
 await page.route('http://gibbon.test/**',route=>{
  const name=path.basename(new URL(route.request().url()).pathname);
  if(['telemetry.js','updater-guard.js','local-flow.js'].includes(name))return route.fulfill({body:'',contentType:'text/javascript'});
  return route.fulfill({body:fs.readFileSync(path.join(root,'ui',name)),contentType:/\.m?js$/.test(name)?'text/javascript':name.endsWith('.css')?'text/css':'text/html'});
 });
 for(const [width,height] of [[144,210],[300,210],[600,420],[300,100]]){
  await page.setViewportSize({width,height});await page.goto('http://gibbon.test/index.html');
  await page.waitForFunction(()=>!document.querySelector('#edit').disabled);
  const before=await page.locator('#memo-surface').boundingBox();
  await page.locator('#edit').click();
  await page.waitForSelector('#feedback:not([hidden])');
  await page.locator('#markdown').fill('수정한 초안\n보존할 문자열');
  const after=await page.locator('#memo-surface').boundingBox();
  assert.deepEqual(after,before,'editing must not grow the native viewport');
  const bounds=await page.evaluate(()=>{
   const rect=id=>{const r=document.getElementById(id).getBoundingClientRect();return{x:r.x,y:r.y,width:r.width,height:r.height,bottom:r.bottom,right:r.right};};
   return {memo:rect('memo-surface'),editor:rect('markdown'),footer:rect('feedback'),scrollWidth:document.documentElement.scrollWidth};
  });
  assert(bounds.editor.height>0);assert(bounds.editor.bottom<=bounds.footer.y+1);
  assert(bounds.footer.bottom<=height);assert(bounds.scrollWidth<=width);
  // Every wrapped action remains reachable in the bounded footer scroll area.
  for(const action of ['cancel','save']){
   const button=page.locator(`#actions [data-action="${action}"]`);
   await button.evaluate(e=>e.scrollIntoView({block:'nearest'}));
   const r=await button.boundingBox();assert(r.x>=0 && r.x+r.width<=width && r.y+r.height<=height);
  }
  assert.equal(await page.locator('#message').textContent(), '');
  assert.equal(await page.locator('#export').isVisible(), false);
  assert.equal(await page.locator('#controls').evaluate(e=>e.getBoundingClientRect().height),22);
  assert.equal(await page.evaluate(()=>document.querySelector('#markdown').getBoundingClientRect().top-document.querySelector('#controls').getBoundingClientRect().bottom),0);
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
    const thumbBounds = await bar.boundingBox();
    await page.mouse.move(thumbBounds.x + 1, thumbBounds.y + 2);
    await page.mouse.down();
    await page.mouse.move(thumbBounds.x + 1, thumbBounds.y + thumbBounds.height - 2, {steps: 8});
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
  await textarea.fill('수정한 초안\n보존할 문자열');
  await page.locator('#markdown').focus();
  await page.locator('#markdown').evaluate(e=>e.setSelectionRange(2,5));
  for(const design of ['white','black','gray']) {
   await page.evaluate(value=>localStorage.setItem('agent-gibbon.memo-design',value),design);
   await page.waitForFunction(value=>getComputedStyle(document.documentElement).getPropertyValue('--background').trim()===({white:'#f8f8f8',black:'#262627',gray:'#dfe2dd'}[value]),design);
   assert.equal(await page.locator('#markdown').inputValue(),'수정한 초안\n보존할 문자열');
   assert.equal(await page.evaluate(()=>document.activeElement.id),'markdown');
   assert.equal(await page.locator('#markdown').evaluate(e=>e.selectionStart),2);
  }
  // Fold only hides the footer; reopening keeps the same draft DOM.
  await page.evaluate(()=>window.editorIdentity=document.getElementById('markdown'));
  await page.evaluate(()=>window.gibbonToggleNote());
  assert.equal(await page.locator('#feedback').isVisible(),false);
  await page.evaluate(()=>window.gibbonToggleNote());
  assert.equal(await page.evaluate(()=>window.editorIdentity===document.getElementById('markdown')),true);
  assert.equal(await page.locator('#markdown').inputValue(),'수정한 초안\n보존할 문자열');
  await page.screenshot({path:path.join(output,`editing-${width}-${height}.png`)});
  await page.evaluate(()=>window.testSaveError=true);
  await page.locator('#actions [data-action="save"]').click();
  await page.waitForFunction(()=>document.querySelector('#message').getAttribute('role')==='alert');
  assert.equal(await page.locator('#markdown').inputValue(),'수정한 초안\n보존할 문자열');
  assert.deepEqual(await page.locator('#memo-surface').boundingBox(),before);
  await page.screenshot({path:path.join(output,`error-${width}-${height}.png`)});
  await page.locator('#actions [data-action="save"]').evaluate(e=>e.scrollIntoView({block:'nearest'}));
  await page.evaluate(()=>window.testSaveError=false);
  if(width===300 && height===210){await page.locator('#markdown').focus();await page.keyboard.press('Control+s');}
  else await page.locator('#actions [data-action="save"]').click();
  await page.waitForFunction(()=>window.testCalls.some(c=>c.name==='save_note'));
  assert.equal(await page.evaluate(()=>window.testNote.markdown),'수정한 초안\n보존할 문자열');
  await page.screenshot({path:path.join(output,`inline-${width}-${height}.png`)});
  results.push({width,height,editorHeight:bounds.editor.height,footerHeight:bounds.footer.height});
 }
 assert.deepEqual(errors,[]);
 console.log(JSON.stringify({browser:browser.version(),results,errors,output}));
} finally {await browser.close();}
