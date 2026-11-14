// Real toolbar markup/CSS in headless Chromium; native IPC is intentionally absent.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
import {memoThemeFor} from '../ui/theme.mjs';
const {chromium} = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
const root = fileURLToPath(new URL('..', import.meta.url));
const output = process.env.GIBBON_SCREENSHOTS;
if (output) fs.mkdirSync(output, {recursive:true});
const browser = await chromium.launch({headless:true});
try {
 const page = await browser.newPage({viewport:{width:600,height:160}});
 await page.route('http://gibbon.test/**', route=>{
  const file=path.join(root,'ui',new URL(route.request().url()).pathname);
  if (/\.m?js$/.test(file)) return route.fulfill({body:'',contentType:'text/javascript'});
  return route.fulfill({body:fs.readFileSync(file),contentType:file.endsWith('.css')?'text/css':'text/html'});
 });
 for (const kind of ['text','image','terminal','web']) {
  for (const design of ['white','gray','black']) {
   await page.goto(`http://gibbon.test/${kind==='terminal'?'terminal':kind==='web'?'web':'index'}.html`);
   await page.evaluate(({theme,kind})=>{
    for(const [key,value] of Object.entries(theme))document.documentElement.style.setProperty(`--${key}`,value);
    if(kind==='image')document.body.classList.add('image-note');
    document.querySelector('#content')?.replaceChildren(document.createTextNode('툴바 Tab 포커스 · '+kind));
   },{theme:memoThemeFor(null,{design}),kind});
   await page.keyboard.press('Tab');
   const focused=page.locator('#controls button:focus-visible');
   assert.equal(await focused.count(),1);
   const before=await focused.boundingBox();
   const state=await focused.evaluate(e=>{const s=getComputedStyle(e);return {color:s.outlineColor,width:s.outlineWidth,style:s.outlineStyle,offset:s.outlineOffset,opacity:s.opacity};});
   assert.match(state.color,/0\.28\)/); assert.equal(state.width,'2px');assert.equal(state.style,'solid');assert.equal(state.offset,'-2px');assert.equal(state.opacity,'1');
   if(output)await page.screenshot({path:path.join(output,`${kind}-${design}.png`)});
   await page.keyboard.press('Tab');assert.equal(await focused.count(),1);
   await page.keyboard.press('Shift+Tab');
   assert.deepEqual(await focused.boundingBox(),before);
   await page.emulateMedia({forcedColors:'active'});
   const forced=await focused.evaluate(e=>getComputedStyle(e).outlineColor);
   assert(!forced.includes('0.28'));assert.notEqual(forced,'rgba(0, 0, 0, 0)');
   await page.emulateMedia({forcedColors:'none'});
   console.log(`PASS ${kind} ${design}: Tab/Shift+Tab, 28% ring, unchanged bounds, forced-colors ${forced}`);
  }
 }
 console.log(`Chromium ${browser.version()}`);
} finally {await browser.close();}
