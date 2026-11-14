// Real manager and memo windows with inline Feedback; IPC mocked.
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
const {default:puppeteer}=await import(process.env.PUPPETEER_MODULE || 'puppeteer-core');
const root=fileURLToPath(new URL('..',import.meta.url));
const out=process.env.APPEARANCE_OUTPUT || '/tmp/gibbon-appearance';fs.mkdirSync(out,{recursive:true});
const browser=await puppeteer.launch({executablePath:process.env.CHROME_BIN || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
const errors=[],external=[],pages=[];
async function pageFor(path,width,height){
 const page=await browser.newPage();pages.push(page);page.setDefaultTimeout(10000);
 for(const method of ['click','focus','screenshot']){const raw=page[method].bind(page);page[method]=async(...args)=>{await page.bringToFront();return raw(...args);};}await page.setViewport({width,height});page.on('pageerror',e=>errors.push(e.message));
 await page.setRequestInterception(true);
 page.on('request',r=>{const u=new URL(r.url());if(u.hostname!=='gibbon.test'){external.push(r.url());return r.abort();}const f=root+'ui'+u.pathname;if(!fs.existsSync(f))return r.respond({status:404,body:''});r.respond({status:200,contentType:/\.(m?js)$/.test(f)?'text/javascript':f.endsWith('.css')?'text/css':'text/html',body:fs.readFileSync(f)});});
 await page.evaluateOnNewDocument(()=>{
  window.calls=[];window.noteColor='#ffeeaa';
  window.__TAURI__={webviewWindow:{getCurrentWebviewWindow:()=>({listen:async()=>()=>{}})},window:{getCurrentWindow:()=>({label:'note-appearance-probe',startResizeDragging:async()=>{}})},event:{listen:async()=>()=>{}},clipboardManager:{writeText:async()=>{}},core:{invoke:async(name,args)=>{
   window.calls.push({name,args});
   if(name==='memo_frame_focus')return true;
   if(name==='telemetry_status')return {detailed:false};
   if(name==='privacy_status')return {terms_accepted:false,terms_skipped:true,sharing:false,allowed:false,notice_ready:true};
   if(name==='manager_notes')return [];
   if(name==='manager_set_theme')return;
   if(name==='skills_list')return {skills:[],feedback:null};
   if(name==='snapshot')return {visible:true,always_on_top:true,highlight_revision:2,note:{edit_on_open:location.search.includes('inline-probe'),title:'컬러 모드 검증',color:window.noteColor,revision:2,markdown:'# 메모 디자인\n보관한 개별 색상',html:'<h1>메모 디자인</h1><p>테마와 컬러 모드</p>',change_html:'<h1>메모 디자인</h1><p>테마와 컬러 모드</p>'}};
   if(name==='fit_note')return true;
   if(name==='poll_fold'||name==='poll_warp')return false;
   if(name==='present_feedback'||name==='content_action'||name==='sync_opacity')return;
   throw Error('Unexpected IPC '+name);
  }}};
 });
 await page.emulateMediaFeatures([{name:'prefers-color-scheme',value:'light'}]);await page.goto('http://gibbon.test/'+path);return page;
}
async function color(page,expected){await page.waitForFunction(x=>getComputedStyle(document.documentElement).getPropertyValue('--background').trim()===x,{polling:100},expected);}
async function systemAlignment(page){
 const groups=await page.$$eval('.theme-options',fields=>fields.map(field=>{
  const labels=[...field.querySelectorAll('label')],system=field.querySelector('.theme-system-option'),prev=labels.at(-2);
  const f=field.getBoundingClientRect(),r=system.getBoundingClientRect(),p=prev.getBoundingClientRect(),css=getComputedStyle(field);
  return {order:labels.map(label=>label.querySelector('input').value),right:r.right,contentRight:f.right-parseFloat(css.paddingRight)-parseFloat(css.borderRightWidth),top:r.top,prevTop:p.top,left:r.left,prevRight:p.right};
 }));
 assert.deepEqual(groups.map(g=>g.order),[['white','gray','black','system'],['white','gray','black','color','system']]);
 for(const g of groups){assert(Math.abs(g.right-g.contentRight)<2,JSON.stringify(g));assert(g.top>=g.prevTop-1);if(Math.abs(g.top-g.prevTop)<2)assert(g.left-g.prevRight>=24);}
}
try {
 const manager=await pageFor('manager.html',944,480);await manager.waitForSelector('#manager-navigation:not([hidden])');await manager.click('#settings-nav');await manager.click('#settings-appearance-nav');
 assert.equal(await manager.$eval('input[name="memo-design"]:checked',e=>e.value),'gray');
 assert.equal((await manager.$$('.memo-sample')).length,2);
 await systemAlignment(manager);
 const memo=await pageFor('index.html',300,240),feedback=await pageFor('index.html?inline-probe',300,240);
 await color(memo,'#dfe2dd');await color(feedback,'#dfe2dd');
 const chooseDesign=async design=>{await manager.$eval(`input[name="memo-design"][value="${design}"]`,e=>e.scrollIntoView());await manager.click(`input[name="memo-design"][value="${design}"]`);};
 const chooseManager=async theme=>{await manager.$eval(`input[name="manager-theme"][value="${theme}"]`,e=>e.scrollIntoView());await manager.click(`input[name="manager-theme"][value="${theme}"]`);};
 for(const [design,bg] of [['white','#f8f8f8'],['gray','#dfe2dd'],['black','#262627'],['system','#dfe2dd'],['color','#ffeeaa']]){
  await chooseDesign(design);await color(memo,bg);await color(feedback,bg);
  const sampleColors=await manager.$$eval('.memo-sample',elements=>elements.map(e=>getComputedStyle(e).backgroundColor));
  const expectedSamples=design==='color'?['rgb(254, 233, 62)','rgb(137, 240, 255)']:Array(2).fill({white:'rgb(248, 248, 248)',gray:'rgb(223, 226, 221)',black:'rgb(38, 38, 39)',system:'rgb(223, 226, 221)'}[design]);
  assert.deepEqual(sampleColors,expectedSamples);
  await manager.$eval('.memo-preview',e=>e.scrollIntoView());await manager.screenshot({path:`${out}/preview-${design}.png`});
  for(const theme of ['white','gray','black','system']){await chooseManager(theme);await color(memo,bg);await color(feedback,bg);}
  await memo.screenshot({path:`${out}/memo-${design}.png`});await feedback.screenshot({path:`${out}/feedback-${design}.png`});
  for(const page of [memo,feedback])assert.equal(await page.evaluate(()=>window.noteColor),'#ffeeaa');
  await memo.reload();await feedback.reload();await color(memo,bg);await color(feedback,bg);
 }
 await manager.reload();await manager.waitForSelector('#manager-navigation:not([hidden])');await manager.click('#settings-nav');await manager.click('#settings-appearance-nav');assert.equal(await manager.$eval('input[name="memo-design"]:checked',e=>e.value),'color');
 await manager.setViewport({width:600,height:360});await systemAlignment(manager);await chooseDesign('gray');await manager.focus('input[name="memo-design"][value="gray"]');await manager.keyboard.press('ArrowRight');
 assert.equal(await manager.$eval('input[name="memo-design"]:checked',e=>e.value),'black');await color(memo,'#262627');await color(feedback,'#262627');assert(await manager.$eval('.settings-content',e=>e.scrollWidth<=e.clientWidth));await manager.$eval('.memo-preview',e=>e.scrollIntoView());assert(await manager.$eval('.memo-preview-grid',e=>e.scrollWidth<=e.clientWidth));await manager.screenshot({path:`${out}/settings-600.png`});
 await memo.focus('#edit');assert.equal(await memo.$eval('#edit',e=>getComputedStyle(e).outlineWidth),'2px');await memo.keyboard.press('Enter');await memo.$eval('#markdown',e=>e.value='보존할 편집 초안');
 await memo.evaluate(()=>window.editorNode=document.getElementById('markdown'));await feedback.focus('#actions button');await feedback.evaluate(()=>window.actionNode=document.querySelector('#actions button'));
 await chooseDesign('white');await chooseManager('gray');await color(memo,'#f8f8f8');await color(feedback,'#f8f8f8');
 await manager.$eval('input[name="manager-theme"][value="system"]',e=>e.scrollIntoView());await manager.click('input[name="manager-theme"][value="system"]');for(const page of pages)await page.emulateMediaFeatures([{name:'prefers-color-scheme',value:'dark'}]);
 await manager.waitForFunction(()=>document.documentElement.dataset.managerTheme==='black',{polling:100});await color(memo,'#f8f8f8');await color(feedback,'#f8f8f8');
 assert.equal(await memo.$eval('#markdown',e=>e.value),'보존할 편집 초안');assert(await memo.evaluate(()=>window.editorNode===document.getElementById('markdown')));assert(await feedback.evaluate(()=>window.actionNode===document.querySelector('#actions button')));assert.equal(await memo.evaluate(()=>document.activeElement.id),'markdown');
 await chooseManager('white');await chooseDesign('system');await color(memo,'#262627');await color(feedback,'#262627');
 assert.deepEqual(await manager.$$eval('.memo-sample',elements=>elements.map(e=>getComputedStyle(e).backgroundColor)),Array(2).fill('rgb(38, 38, 39)'));
 for(const page of pages)await page.emulateMediaFeatures([{name:'prefers-color-scheme',value:'light'}]);await color(memo,'#dfe2dd');await color(feedback,'#dfe2dd');assert.equal(await manager.$eval('html',e=>e.dataset.managerTheme),'white');
 assert.deepEqual(await manager.$$eval('.memo-sample',elements=>elements.map(e=>getComputedStyle(e).backgroundColor)),Array(2).fill('rgb(223, 226, 221)'));
 await chooseDesign('gray');await manager.close();for(const page of [memo,feedback])await page.emulateMediaFeatures([{name:'prefers-color-scheme',value:'dark'}]);await color(memo,'#dfe2dd');await color(feedback,'#dfe2dd');
 for(const page of [memo,feedback])assert.equal(await page.evaluate(()=>window.calls.some(c=>['save_note','hide_note','close_note','toggle_pin','manual_resize','privacy_save'].includes(c.name))),false);
 assert.deepEqual(errors,[]);assert.deepEqual(external,[]);
 console.log('PASS: 5 independent memo designs × 4 manager themes, default gray, original color and reload persistence, optional system day/night tracking, OS/manager independence, inline Feedback in memo windows, 2 live fixed sample memos in all designs, 944/600px last-system order and right alignment, preview/radio keyboard/access, editor draft/focus and Feedback DOM preservation, no note/consent writes or external requests.');
}finally{await browser.close();}
