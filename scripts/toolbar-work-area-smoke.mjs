// Real Chromium pointer/double-click delivery to production toolbar; native IPC mocked.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
const {chromium} = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
const root=fileURLToPath(new URL('..',import.meta.url));
const browser=await chromium.launch({headless:true});
try {
 const page=await browser.newPage({viewport:{width:900,height:600}}), errors=[];
 let fullWeb=false;
 page.on('pageerror',e=>errors.push(e.message));
 await page.addInitScript(()=>{
   window.testCalls=[];
   window.__TAURI__={core:{invoke:async name=>{window.testCalls.push(name);if(name==='web_chrome')return {url:null,welcome:true,always_on_top:true};return false;}},webviewWindow:{getCurrentWebviewWindow:()=>({listen(){}})}};
 });
 await page.route('http://gibbon.test/**',route=>{
   const file=path.join(root,'ui',new URL(route.request().url()).pathname);
   if(/\.js$/.test(file)&&!file.endsWith('/toolbar.js')&&!file.endsWith('/overlay-scrollbar.js')&&!(fullWeb&&/\/web(?:-chrome)?\.js$/.test(file))) return route.fulfill({body:'',contentType:'text/javascript'});
   return route.fulfill({body:fs.readFileSync(file),contentType:file.endsWith('.css')?'text/css':/\.m?js$/.test(file)?'text/javascript':'text/html'});
 });
 for(const kind of ['text','image','terminal','web']) {
   await page.goto(`http://gibbon.test/${kind==='web'?'web':kind==='terminal'?'terminal':'index'}.html`);
   await page.waitForFunction(()=>typeof window.gibbonToolbarFold==='function');
   if(kind==='image') await page.evaluate(()=>document.body.classList.add('image-note'));
   const row=await page.locator('#controls').boundingBox();
   const point=await page.evaluate(()=>{
     const row=document.getElementById('controls'),r=row.getBoundingClientRect(),y=r.y+10;
     for(let x=r.right-60;x>r.left+10;x-=1) {
       const hit=document.elementFromPoint(x,y);
       if(hit?.closest('#controls')&&!hit.closest('button,#chrome-title')&&
          !document.elementFromPoint(x-18,y)?.closest('button,#chrome-title'))return {x,y};
     }
     return null;
   });
   assert.ok(point,`${kind}: blank toolbar exists`);
   await page.mouse.click(point.x,point.y);
   assert.deepEqual(await page.evaluate(()=>testCalls.filter(n=>n==='chrome_work_area'||n==='chrome_drag')),[]);
   await page.mouse.dblclick(point.x,point.y);
   assert.deepEqual(await page.evaluate(()=>testCalls.filter(n=>n==='chrome_work_area'||n==='chrome_drag')),['chrome_work_area']);
   await page.mouse.move(point.x,point.y);await page.mouse.down();
   await page.mouse.move(point.x-2,point.y);assert.equal((await page.evaluate(()=>testCalls.filter(n=>n==='chrome_work_area'||n==='chrome_drag'))).length,1);
   await page.mouse.move(point.x-15,point.y,{steps:3});await page.mouse.up();
   assert.deepEqual(await page.evaluate(()=>testCalls.filter(n=>n==='chrome_work_area'||n==='chrome_drag')),['chrome_work_area','chrome_drag']);
   const before=await page.evaluate(()=>testCalls.filter(n=>n==='chrome_work_area'||n==='chrome_drag').length);
   await page.locator('#close').dblclick();assert.equal(await page.evaluate(()=>testCalls.filter(n=>n==='chrome_work_area'||n==='chrome_drag').length),before);
   await page.evaluate(()=>gibbonToolbarFold(true));
   await page.locator('#chrome-title').dblclick();assert.equal(await page.evaluate(()=>testCalls.filter(n=>n==='chrome_work_area'||n==='chrome_drag').length),before);
   console.log(`PASS ${kind}: actual stationary click/dblclick/drag/buttons/title`);
 }
 fullWeb=true;
 await page.goto('http://gibbon.test/web.html');
 await page.waitForFunction(()=>window.testCalls.includes('web_chrome')&&typeof window.gibbonToolbarFold==='function');
 for(const state of ['welcome','external-content']) {
   if(state==='external-content') {
     await page.locator('#web-url').fill('https://example.com/');
     await page.locator('#run').click();
     await page.waitForFunction(()=>document.querySelector('main').hidden);
     // Model the separate remote surface below the native chrome. The OS child
     // WebView's actual bounds and IPC isolation require native verification.
     await page.evaluate(()=>{
       const frame=document.createElement('iframe');frame.srcdoc='<p>External content fixture</p>';
       Object.assign(frame.style,{position:'fixed',left:'3px',top:'30px',width:'894px',height:'567px',border:'0'});
       document.body.append(frame);
     });
   }
   const before=await page.evaluate(()=>testCalls.filter(n=>n==='chrome_work_area').length);
   await page.mouse.dblclick(80,10);
   assert.equal(await page.evaluate(()=>testCalls.filter(n=>n==='chrome_work_area').length),before+1);
   console.log(`PASS full web scripts ${state}: initialization and blank toolbar double-click`);
 }
 assert.deepEqual(errors,[]);
} finally {await browser.close();}
