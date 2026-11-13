// Render the trusted chrome with mocked native commands; remote content is not loaded.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
const {chromium} = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
const root = fileURLToPath(new URL('..', import.meta.url));
const browser = await chromium.launch({headless:true});
const output = process.env.GIBBON_SCREENSHOTS;
if (output) fs.mkdirSync(output, {recursive:true});
try {
 const page = await browser.newPage(); const errors=[];
 page.on('pageerror', e=>errors.push(e.message));
 await page.addInitScript(()=>{
  let folded=false, pinned=true;
  window.__TAURI__={core:{invoke:async(name,args)=>{
   if(name==='chrome_fold') return folded=!folded;
   if(name==='web_chrome') {if(args.action==='pin') pinned=!pinned;return {url:null,welcome:true,always_on_top:pinned};}
   if(name==='web_run') return {};
  }},event:{listen:()=>Promise.resolve(()=>{})},window:{getCurrentWindow:()=>({label:'test'})},webviewWindow:{getCurrentWebviewWindow:()=>({listen:()=>{}})}};
 });
 await page.route('http://gibbon.test/**', route=>{
  const file=path.join(root,'ui',new URL(route.request().url()).pathname);
  if(/(terminal|terminal-ime)\.js$/.test(file)||file.includes('/vendor/')) return route.fulfill({body:'',contentType:'text/javascript'});
  return route.fulfill({body:fs.readFileSync(file),contentType:file.endsWith('.css')?'text/css':/\.m?js$/.test(file)?'text/javascript':'text/html'});
 });
 for(const design of ['gray','white','black','color','system']) {
  for(const width of [144,240,600]) {
   await page.setViewportSize({width,height:400});
   await page.goto('http://gibbon.test/web.html');
   await page.evaluate(design=>{localStorage.setItem('agent-gibbon.memo-design',design);dispatchEvent(new StorageEvent('storage',{key:'agent-gibbon.memo-design',storageArea:localStorage}));},design);
   await page.waitForFunction(()=>!!window.gibbonToolbarFold);
   const geometry=()=>page.locator('#controls').evaluate(row=>({height:row.offsetHeight,buttons:[...row.querySelectorAll('#pin,#toolbar-toggle,#close')].map(b=>({id:b.id,x:b.getBoundingClientRect().x,y:b.getBoundingClientRect().y,w:b.offsetWidth,h:b.offsetHeight,r:getComputedStyle(b).borderRadius,p:getComputedStyle(b).padding,icon:b.querySelector('svg').getBoundingClientRect().width}))}));
   assert.equal(await page.locator('#chrome-title').getAttribute('title'),'웹');
   const expanded=await geometry(); assert.equal(expanded.height,30);
   for(const b of expanded.buttons){assert.equal(b.w,24);assert.equal(b.h,22);assert.equal(b.icon,12);assert.equal(b.r,'3px');assert.equal(b.p,'0px');}
   const color=await page.locator('#controls').evaluate(e=>getComputedStyle(e).color);
   assert.equal(color,await page.locator('body').evaluate(e=>getComputedStyle(e).color));
   await page.locator('#pin').click();assert.equal(await page.locator('#pin').getAttribute('aria-pressed'),'false');
   assert.equal(await page.locator('#pin svg').evaluate(e=>getComputedStyle(e).opacity),'0.45');
   await page.keyboard.press('Tab'); await page.locator('#toolbar-toggle').focus(); assert.notEqual(await page.locator('#toolbar-toggle').evaluate(e=>getComputedStyle(e).outlineStyle),'none');
   await page.keyboard.press('Enter');
   assert.equal(await page.locator('main').isVisible(),false);
   const collapsed=await geometry(); assert.equal(collapsed.height,22);
   for(const id of ['toolbar-toggle','close']) {const a=expanded.buttons.find(b=>b.id===id),b=collapsed.buttons.find(b=>b.id===id);assert.equal(a.x,b.x);assert.equal(b.y,1);}
   await page.evaluate(()=>{document.getElementById('title').textContent='https://example.com/a-long-page-title';});
   await page.waitForFunction(()=>document.getElementById('chrome-title').title.includes('a-long-page-title'));
   await page.locator('#close').hover();const hovered=await page.locator('#close').evaluate(e=>getComputedStyle(e).backgroundColor);assert.notEqual(hovered,'rgba(0, 0, 0, 0)');
   if(output&&width===240&&['gray','black'].includes(design))await page.screenshot({path:path.join(output,`${design}-collapsed.png`)});
   await page.locator('#toolbar-toggle').click();assert.equal(await page.locator('main').isVisible(),true);
   // Remote-page chrome keeps the row while Welcome is hidden.
   await page.evaluate(()=>{document.querySelector('main').hidden=true;});
   assert.equal(await page.locator('main').isVisible(),false);assert.equal((await geometry()).height,30);
   if(output&&width===600&&['gray','black'].includes(design)) {await page.evaluate(()=>document.querySelector('main').hidden=false);await page.screenshot({path:path.join(output,`${design}-welcome.png`)});}
   console.log(`PASS ${design} ${width}: slots, fold, title, pin, focus, hover, remote chrome`);
  }
 }
 // Compare all common button geometry directly with the terminal toolbar.
 await page.goto('http://gibbon.test/terminal.html');
 const terminal=await page.locator('#close').evaluate(e=>[e.offsetWidth,e.offsetHeight,getComputedStyle(e).borderRadius]);
 await page.goto('http://gibbon.test/web.html');
 assert.deepEqual(await page.locator('#close').evaluate(e=>[e.offsetWidth,e.offsetHeight,getComputedStyle(e).borderRadius]),terminal);
 assert.deepEqual(errors,[]);
} finally {await browser.close();}
