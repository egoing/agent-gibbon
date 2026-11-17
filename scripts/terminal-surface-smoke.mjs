// Real xterm rendering, buffer and input; only native IPC is stubbed.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
const {chromium}=await import(process.env.PLAYWRIGHT_MODULE||'playwright');
const root=fileURLToPath(new URL('..',import.meta.url));
const out=process.env.SURFACE_OUTPUT||'/tmp/gibbon176';fs.mkdirSync(out,{recursive:true});
const browser=await chromium.launch({channel:"chrome"});
const reports=[],errors=[];
try {
 for(const scale of [1,2]) {
 const context=await browser.newContext({deviceScaleFactor:scale});
 await context.route('http://gibbon.test/**',r=>{const f=path.join(root,'ui',new URL(r.request().url()).pathname);return fs.existsSync(f)?r.fulfill({body:fs.readFileSync(f),contentType:/\.(m?js)$/.test(f)?'text/javascript':f.endsWith('.css')?'text/css':'text/html'}):r.fulfill({status:404,body:''});});
 await context.addInitScript(()=>{
  const info={id:'terminal-176',kind:'terminal',title:'176 surface',last_path:'/tmp'};
  if(location.search.includes('restore'))info.restore_plan='echo restored';
  window.calls=[];
  window.__TAURI_INTERNALS__={metadata:{currentWindow:{label:'terminal-176'}}};
  window.__TAURI__={webviewWindow:{getCurrentWebviewWindow:()=>({label:'terminal-176',listen:async()=>()=>{}})},event:{listen:async()=>()=>{}},window:{getCurrentWindow:()=>({label:'terminal-176',startDragging:async()=>{},startResizeDragging:async()=>{}})},core:{Channel:class{},invoke:async(name,args)=>{
   window.calls.push({name,args});
   if(name==='memo_frame_focus')return true;
   if(name==='terminal_welcome')return {info,home:'/tmp',location:{path:'/tmp',name:'tmp',parent:'/',command:'cd /tmp'},folders:[],choices:[{id:'shell',label:'셸',command:'/bin/zsh'}]};
   return info;
  }}};
 });
 const page=await context.newPage();page.on('pageerror',e=>errors.push(e.message));
 for(const [width,height] of [[320,180],[640,400],[421,263]])for(const theme of ['gray','white','black']){
 await page.setViewportSize({width,height});await page.goto('http://gibbon.test/terminal.html');
 await page.evaluate(theme=>{localStorage.setItem('agent-gibbon.memo-design',theme);location.reload();},theme);
 await page.waitForSelector('#run-task');
 assert.equal(await page.locator('#rename-form').count(),0);
 if(height===180){
  const welcomeBar=page.locator('#welcome-scrollbar');
  await page.waitForFunction(()=>!document.querySelector('#welcome-scrollbar').hidden);
  assert.equal(await page.locator('#welcome').evaluate(e=>getComputedStyle(e).scrollbarWidth),'none');
  await welcomeBar.focus();await page.keyboard.press('End');
  assert(await page.locator('#welcome').evaluate(e=>e.scrollTop>0));
  await page.keyboard.press('Home');assert.equal(await page.locator('#welcome').evaluate(e=>e.scrollTop),0);
  await page.screenshot({path:`${out}/welcome-${theme}-${scale}x.png`,omitBackground:true});
 }
 await page.locator('#run-task').click();
 await page.waitForFunction(()=>document.querySelector('#welcome-scrollbar').hidden);

 await page.evaluate(()=>new Promise(done=>terminal.write(Array.from({length:180},(_,i)=>`line ${i} output\r\n`).join(''),done)));
 await page.waitForFunction(()=>!document.querySelector('#memo-scrollbar').hidden);
 await page.waitForTimeout(150);
 const g=await page.evaluate(()=>{
  const rect=s=>{const r=document.querySelector(s).getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height,bottom:r.bottom,right:r.right};};
  return {screen:rect('.xterm-screen'),host:rect('#terminal'),controls:rect('#controls'),rows:terminal.rows,cols:terminal.cols,base:terminal.buffer.active.baseY,viewport:terminal.buffer.active.viewportY};
 });
 assert.equal(await page.locator('.xterm-viewport').evaluate(e=>getComputedStyle(e).backgroundColor),'rgb(32, 37, 46)');
 assert.equal(g.controls.height,22);assert(g.screen.bottom<=g.host.bottom-2.9,JSON.stringify(g));assert(g.screen.right<=g.host.right-2.9,JSON.stringify(g));assert(g.host.right-3-g.screen.right<g.screen.width/g.cols+.1,JSON.stringify(g));assert(g.host.bottom-3-g.screen.bottom<g.screen.height/g.rows+.1);
 const bar=page.locator('#memo-scrollbar');await bar.focus();await page.keyboard.press('Home');await page.waitForFunction(()=>terminal.buffer.active.viewportY===0);await page.keyboard.press('End');await page.waitForFunction(()=>terminal.buffer.active.viewportY===terminal.buffer.active.baseY);
 // Thumb drag moves the buffer without producing PTY input or native window drag.
 await bar.hover();const b=await bar.boundingBox();await page.mouse.move(b.x+b.width-1,b.y+b.height-5);await page.mouse.down();await page.mouse.move(b.x+b.width-1,b.y+5,{steps:8});await page.mouse.up();
 assert(await page.evaluate(()=>terminal.buffer.active.viewportY<terminal.buffer.active.baseY));
 await page.evaluate(()=>terminal.scrollToTop());await page.mouse.move(100,80);await page.mouse.wheel(0,300);await page.waitForFunction(()=>terminal.buffer.active.viewportY>0);
 await page.evaluate(()=>terminal.scrollToBottom());await bar.hover();await page.screenshot({path:`${out}/scrollback-${width}-${theme}-${scale}x.png`,omitBackground:true});
 await page.evaluate(()=>new Promise(done=>terminal.write('\x1b[?1049h\x1b[2J\x1b[H TUI top\x1b['+terminal.rows+';1H TUI last row',done)));
 await page.waitForFunction(()=>document.querySelector('#memo-scrollbar').hidden);
 assert.equal(await page.evaluate(()=>terminal.buffer.active.type),'alternate');
 await page.locator('#terminal').click();await page.keyboard.press('ArrowUp');
 assert(await page.evaluate(()=>window.calls.some(c=>c.name==='terminal_write'&&c.args.data==='\x1b[A')));
 await page.screenshot({path:`${out}/terminal-${width}-${theme}-${scale}x.png`,omitBackground:true});
 await page.evaluate(()=>new Promise(done=>terminal.write('\x1b[?1049l',done)));await page.waitForFunction(()=>!document.querySelector('#memo-scrollbar').hidden);
 assert.equal(await page.evaluate(()=>terminal.buffer.active.baseY),g.base);
 if(width===421&&theme==='gray'){
  for(let i=0;i<4;i++){
   await page.locator('#font-size').click();await page.waitForTimeout(120);
   const f=await page.evaluate(()=>{const s=document.querySelector('.xterm-screen').getBoundingClientRect(),h=document.querySelector('#terminal').getBoundingClientRect();return {bottom:s.bottom,limit:h.bottom-3,right:s.right,end:h.right-3};});assert(f.bottom<=f.limit+.1&&f.right<=f.end+.1,JSON.stringify(f));
  }
  await page.evaluate(()=>new Promise(done=>terminal.write('\x1b[?1049h\x1b[?1000h\x1b[?1006h',done)));
  const n=await page.evaluate(()=>window.calls.filter(c=>c.name==='terminal_write').length);
  await page.mouse.move(100,80);await page.mouse.wheel(0,120);
  await page.waitForFunction(n=>window.calls.filter(c=>c.name==='terminal_write').length>n,n);
  assert(await page.evaluate(n=>window.calls.filter(c=>c.name==='terminal_write').slice(n).some(c=>c.args.data.startsWith('\x1b[<65')),n));
  await page.evaluate(()=>new Promise(done=>terminal.write('\x1b[?1000l\x1b[?1006l\x1b[?1049l',done)));
 }

 const applied=await page.evaluate(()=>{
  const {cols,rows}=window.calls.filter(c=>c.name==='terminal_start'||c.name==='terminal_resize').at(-1).args;
  return {cols,rows,screenCols:terminal.cols,screenRows:terminal.rows};
 });
 assert.equal(applied.cols,applied.screenCols);assert.equal(applied.rows,applied.screenRows);
 // Actual capture layout styles; keep an intrinsic 16:9 image and no toolbar gap.
 await page.evaluate(()=>{document.body.classList.add('image-note');document.querySelector('header').replaceWith(document.querySelector('#controls'));document.querySelector('#terminal').hidden=true;const v=document.createElement('main');v.id='viewport';v.innerHTML='<article id="content"><img alt="fixture" src="data:image/svg+xml,%3Csvg xmlns=%22http://www.w3.org/2000/svg%22 width=%221600%22 height=%22900%22%3E%3Crect width=%221600%22 height=%22900%22 fill=%22%235a8%22/%3E%3C/svg%3E"></article>';document.querySelector('#memo-surface').append(v);});
 const image=await page.locator('#viewport').evaluate(e=>({top:e.getBoundingClientRect().top,bottom:document.querySelector('#controls').getBoundingClientRect().bottom,padding:getComputedStyle(e).padding,fit:getComputedStyle(e.querySelector('img')).objectFit}));assert.equal(image.top,image.bottom);assert.equal(image.padding,'0px');assert.equal(image.fit,'contain');
 const surface=await page.locator('#memo-surface').evaluate(element=>{
   const style=getComputedStyle(element);
   return [style.backgroundColor,style.borderLeftWidth,style.borderRightWidth,style.borderBottomWidth];
 });
 assert.deepEqual(surface,['rgba(0, 0, 0, 0)','0px','0px','0px']);
 await page.screenshot({path:`${out}/capture-${width}-${theme}-${scale}x.png`,omitBackground:true});
 if(width===421&&theme==='black'){
  await page.setViewportSize({width:640,height:382});
  await page.waitForTimeout(80);
  for(const below of [false,true]){
   await page.evaluate(below=>document.body.classList.toggle('capture-toolbar-below',below),below);
   const exact=await page.locator('#viewport').evaluate(e=>{const v=e.getBoundingClientRect(),i=e.querySelector('img'),r=i.getBoundingClientRect();return {x:v.x,y:v.y,width:v.width,height:v.height,imageWidth:r.width,imageHeight:r.height,aspect:i.naturalWidth/i.naturalHeight};});
   assert.equal(exact.x,0);assert.equal(exact.y,below?0:22);assert.equal(exact.width,640);assert.equal(exact.height,360);assert.equal(exact.imageWidth/exact.imageHeight,exact.aspect);
   await page.screenshot({path:`${out}/capture-exact-${below?'below':'above'}-${scale}x.png`,omitBackground:true});
  }
 }
 reports.push({width,height,theme,scale,...g,image});
 }
 await page.setViewportSize({width:320,height:180});
 await page.goto('http://gibbon.test/terminal.html?restore');
 await page.waitForSelector('#restore-panel:not([hidden])');
 await page.waitForFunction(()=>document.querySelector('#welcome-scrollbar').hidden);
 await page.locator('#restore-cancel').click();
 await page.waitForSelector('#welcome-panel:not([hidden])');
 await page.waitForFunction(()=>!document.querySelector('#welcome-scrollbar').hidden);
 assert.equal(await page.locator('#restore-panel').isVisible(),false);
 await context.close();
 }
 assert.deepEqual(errors,[]);fs.writeFileSync(`${out}/results.json`,JSON.stringify(reports,null,2));console.log(`PASS ${reports.length} theme/size/scale combinations; normal/alternate buffer, thumb, wheel, keyboard, fit/resize, capture containment`);
} finally {await browser.close();}
