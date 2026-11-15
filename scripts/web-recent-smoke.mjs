// Real Welcome assets, mocked native calls. Filesystem persistence/security is
// covered by Rust tests; this checks rendered interaction without native GUI.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
const {chromium}=await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
const root=fileURLToPath(new URL('..',import.meta.url));
const out=process.env.GIBBON_SCREENSHOTS;
if(out)fs.mkdirSync(out,{recursive:true});
const browser=await chromium.launch({headless:true});
try {
 const page=await browser.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));
 let urls=[],calls=[],failDelete=false;
 await page.exposeFunction('mockNative',async(name,args)=>{
  if(name==='web_chrome') return {welcome:true,always_on_top:true};
  if(name==='web_recent') {
   if(args?.remove) {if(failDelete) throw Error('삭제 실패');urls=urls.filter(u=>u!==args.remove);}
   return urls;
  }
  if(name==='web_run') {calls.push(args.url);throw Error('연결 오류');}
  return false;
 });
 await page.addInitScript(()=>{window.__TAURI__={core:{invoke:(name,args)=>window.mockNative(name,args)},webviewWindow:{getCurrentWebviewWindow:()=>({listen:()=>{}})}};});
 await page.route('http://gibbon.test/**',route=>{
  const file=path.join(root,'ui',new URL(route.request().url()).pathname);
  return route.fulfill({body:fs.readFileSync(file),contentType:file.endsWith('.css')?'text/css':/\.m?js$/.test(file)?'text/javascript':'text/html'});
 });
 const settle=()=>page.waitForFunction(()=>document.getElementById('web-recent-list').children.length===10);
 for(const [width,height] of [[240,160],[320,240],[600,400]]) {
  urls=Array.from({length:10},(_,i)=>`https://example.com/${i}/`);
  urls[1]='https://example.com/a-very-long-path-to-check-ellipsis?value=%3Cimg%3E';calls=[];
  await page.setViewportSize({width,height});await page.goto('http://gibbon.test/web.html');await settle();
  assert.equal(await page.getByRole('textbox',{name:'웹 주소',exact:true}).count(),1);
  assert.equal(await page.getByRole('button',{name:'이동',exact:true}).count(),1);
  assert.equal(await page.locator('label,#hint').count(),0);
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
  const open=page.locator('.recent-open').first();await open.focus();await page.keyboard.press('Enter');
  await page.waitForFunction(()=>document.getElementById('status').textContent.includes('연결 오류'));
  assert.deepEqual(calls,['https://example.com/0/']);
  assert.equal(await page.locator('#web-url').inputValue(),'https://example.com/0/');
  failDelete=true;await page.locator('.recent-remove').first().click();
  await page.waitForFunction(()=>document.getElementById('status').textContent.includes('삭제 실패'));
  assert.equal(await page.locator('.recent-open').count(),10);failDelete=false;
  await page.locator('.recent-remove').first().focus();await page.keyboard.press('Enter');
  await page.waitForFunction(()=>document.getElementById('web-recent-list').children.length===9);
  assert.equal(await page.locator('.recent-remove').first().evaluate(e=>e===document.activeElement),true);
  await page.reload();await page.waitForFunction(()=>document.getElementById('web-recent-list').children.length===9);
  if(out)await page.screenshot({path:path.join(out,`recent-${width}.png`)});
  urls=[urls[0]];await page.reload();await page.waitForFunction(()=>document.getElementById('web-recent-list').children.length===1);
  await page.locator('.recent-remove').focus();await page.keyboard.press('Enter');
  await page.waitForFunction(()=>document.getElementById('web-recent').hidden);
  assert.equal(await page.locator('#web-url').evaluate(e=>e===document.activeElement),true);
  console.log(`PASS ${width}: recent list, direct Enter navigation, error, delete focus, reload, empty, overflow`);
 }
 assert.deepEqual(errors,[]);
} finally {await browser.close();}
