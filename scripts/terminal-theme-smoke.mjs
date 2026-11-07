// Actual terminal HTML/CSS/xterm in Chromium; only local Tauri IPC is mocked.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
const {chromium}=await import(process.env.PLAYWRIGHT_MODULE||'playwright');
const root=fileURLToPath(new URL('..',import.meta.url));
const out=process.env.TERMINAL_THEME_OUTPUT||'/tmp/gibbon-terminal-themes';fs.mkdirSync(out,{recursive:true});
const browser=await chromium.launch({headless:true});const errors=[],external=[];
try {
 const context=await browser.newContext();
 await context.route('http://gibbon.test/**',route=>{
  const f=path.join(root,'ui',new URL(route.request().url()).pathname);
  return fs.existsSync(f)?route.fulfill({body:fs.readFileSync(f),contentType:/\.(m?js)$/.test(f)?'text/javascript':f.endsWith('.css')?'text/css':'text/html'}):route.fulfill({status:404,body:''});
 });
 await context.route(/^(?!http:\/\/gibbon\.test\/).*/,route=>{external.push(route.request().url());return route.abort();});
 await context.addInitScript(()=>{
  const folder=path=>({path,name:path.split('/').at(-1),parent:'/Users/demo',command:`cd -- '${path}'`});
  const info={id:'terminal-93001',kind:'terminal',title:'테마 대비 검증',last_path:'/Users/demo/project'};window.calls=[];
  window.__TAURI__={window:{getCurrentWindow:()=>({startDragging:async()=>{},startResizeDragging:async()=>{}})},core:{Channel:class{},invoke:async(name,args)=>{
   window.calls.push({name,args});
   if(name==='terminal_welcome')return {info,home:'/Users/demo',location:folder(info.last_path),folders:[folder(info.last_path),folder('/Users/demo/other')],choices:[{id:'codex',label:'Codex',command:"'/usr/local/bin/codex'"},{id:'shell',label:'셸',command:"'/bin/zsh' -i"}]};
   if(name==='terminal_rename')info.title=args.title;
   return info;
  }}};
 });
 const writer=await context.newPage();await writer.goto('http://gibbon.test/tokens.css');
 const page=await context.newPage();page.on('pageerror',e=>errors.push(e.message));
 async function inspect(){return page.evaluate(()=>{
  function bg(el){const s=getComputedStyle(el);return s.backgroundColor==='rgba(0, 0, 0, 0)'&&el.parentElement?bg(el.parentElement):s.backgroundColor;}
  function lum(color){const c=color.match(/[\d.]+/g).slice(0,3).map(Number).map(v=>{v/=255;return v<=.04045?v/12.92:((v+.055)/1.055)**2.4;});return c[0]*.2126+c[1]*.7152+c[2]*.0722;}
  const results=[];
  for(const selector of ['#choose-folder','#recent-folders .select-row[aria-pressed="true"]','.folder-parent','#home-hint','#recommendations .select-row[aria-pressed="true"]','#run-task','#command-preview','#title-details summary','#rename-title','#rename-form button','#rename-title::placeholder']){
   const pseudo=selector.endsWith('::placeholder'),el=document.querySelector(selector.replace('::placeholder',''));const s=getComputedStyle(el,pseudo?'::placeholder':null),a=lum(s.color),b=lum(bg(el));results.push({selector,foreground:s.color,background:bg(el),contrast:(Math.max(a,b)+.05)/(Math.min(a,b)+.05)});
  }
  return {results,background:getComputedStyle(document.documentElement).getPropertyValue('--background').trim(),overflow:document.querySelector('#welcome').scrollWidth>document.querySelector('#welcome').clientWidth};
 });}
 const reports=[];
 for(const [width,height] of [[720,700],[320,240]]){
  await page.setViewportSize({width,height});await page.goto('http://gibbon.test/terminal.html');await page.waitForFunction(()=>document.querySelectorAll('#recommendations button').length===2);
  await page.locator('#title-details').evaluate(e=>e.open=true);await page.locator('#rename-title').fill('유지할 제목 초안');await page.locator('#recommendations button').nth(1).click();
  await page.evaluate(()=>{window.savedNodes=[document.querySelector('#rename-title'),document.querySelector('#recommendations button')];});
  for(const [design,dark,expected] of [['gray',false,'#dfe2dd'],['white',false,'#f8f8f8'],['black',false,'#262627'],['color',false,'#20252e'],['system',true,'#262627'],['system',false,'#dfe2dd'],['gray',true,'#dfe2dd']]){
   await page.emulateMedia({colorScheme:dark?'dark':'light'});await writer.evaluate(design=>localStorage.setItem('agent-gibbon.memo-design',design),design);
   // Wait for actual cross-document storage/media updates, not a manual style injection.
   if(design==='gray'&&reports.length===0){}else await page.waitForFunction(expected=>getComputedStyle(document.documentElement).getPropertyValue('--background').trim()===expected,expected);
   const report=await inspect();for(const result of report.results)assert(result.contrast>=4.5,JSON.stringify({design,width,...result}));assert.equal(report.overflow,false);assert.equal(report.background,expected);
   assert.equal(await page.locator('#rename-title').inputValue(),'유지할 제목 초안');assert.equal(await page.locator('#recommendations button').nth(1).getAttribute('aria-pressed'),'true');assert(await page.evaluate(()=>window.savedNodes[0]===document.querySelector('#rename-title')&&window.savedNodes[1]===document.querySelector('#recommendations button')));
   await page.locator('#run-task').focus();await page.keyboard.press('Tab');assert.equal(await page.evaluate(()=>document.activeElement.id),'command-preview');assert.equal(await page.locator('#command-preview').evaluate(e=>getComputedStyle(e).outlineWidth),'2px');
   await page.locator('#choose-folder').scrollIntoViewIfNeeded();await page.screenshot({path:`${out}/${width}-${design}-${dark?'dark':'light'}.png`});reports.push({width,height,design,dark,...report});
  }
  assert.equal(await page.evaluate(()=>window.calls.some(c=>c.name==='terminal_start')),false);
  await page.locator('#run-task').click();assert.equal(await page.locator('#welcome').isVisible(),false);assert.equal(await page.evaluate(()=>window.calls.filter(c=>c.name==='terminal_start').length),1);
  assert.deepEqual(await page.evaluate(()=>terminal.options.theme),{background:'#20252e',foreground:'#f0f0f0'});
 }
 assert.deepEqual(errors,[]);assert.deepEqual(external,[]);fs.writeFileSync(`${out}/results.json`,JSON.stringify(reports,null,2));console.log(`PASS ${reports.length} theme/size cases, actual text contrast, storage/media switching, draft/selection/DOM preservation, keyboard focus, Run and fixed xterm palette`);
} finally {await browser.close();}
