// Real HTML/CSS/editor/xterm; trusted native IPC is mocked. No physical input.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
import {colorPalette,contrastRatio} from '../ui/theme.mjs';
const {chromium}=await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
const root=fileURLToPath(new URL('../ui/',import.meta.url));
const out='docs/verification/color-palette';fs.mkdirSync(out,{recursive:true});
const browser=await chromium.launch({headless:true,channel:'chrome'});
const results=[],errors=[];
try {
 for(const kind of ['text','image','terminal','web']) {
  const page=await browser.newPage({viewport:{width:320,height:300}});page.on('pageerror',e=>errors.push(e.message));page.setDefaultTimeout(10000);
  await page.addInitScript(({kind,color})=>{
   localStorage.setItem('jajan.memo-design','color');window.listeners={};
   const info={kind:'terminal',color,title:'컬러 테마 예제',last_path:'/tmp/palette',welcome:true,always_on_top:false};
   window.note={color,title:info.title,revision:1,markdown:'# 컬러 메모\n\n본문과 링크 · 기본 글자\n\n- [ ] 할 일',html:'<h1>컬러 메모</h1><p>본문과 링크 · 기본 글자</p>',capture:kind==='image'?{width:200,height:100}:null};
   window.__TAURI__={window:{getCurrentWindow:()=>({label:'note-palette',startResizeDragging:async()=>{},startDragging:async()=>{}})},webviewWindow:{getCurrentWebviewWindow:()=>({listen:async()=>()=>{}})},clipboardManager:{writeText:async()=>{throw Error('예제 피드백 · 원문은 유지됩니다');}},event:{listen:async(name,fn)=>{window.listeners[name]=fn;return()=>{};}},core:{Channel:class{},invoke:async(name,args)=>{
    if(name==='copy_note_image')throw Error('예제 피드백 · 원본은 유지됩니다');
    if(name==='terminal_folder')throw Error('예제 안내 · 작업 위치를 확인하세요');
    if(name==='web_run')throw Error('연결 실패 · 입력은 보존됩니다');
    if(name==='snapshot')return{visible:true,folded:false,always_on_top:false,note:window.note};
    if(name==='memo_frame_focus'||name==='fit_note')return true;
    if(name==='poll_fold'||name==='poll_warp')return false;
    if(name==='terminal_welcome')return{info,home:'/tmp',location:{path:'/tmp/palette',name:'palette',parent:'/tmp',command:"cd -- '/tmp/palette'"},folders:[],choices:[{id:'shell',label:'셸',command:'/bin/zsh -i'}]};
    if(name==='terminal_info'||name==='web_chrome')return info;
    if(name==='web_recent')return['https://example.com/guide'];
    if(name==='note_image')return 'data:image/svg+xml,%3Csvg xmlns="http://www.w3.org/2000/svg" width="200" height="100"%3E%3Crect width="200" height="100" fill="%2335688a"/%3E%3C/svg%3E';
   }}};
  },{kind,color:colorPalette[0].background});
  await page.route('http://jajan.test/**',route=>{
   const name=new URL(route.request().url()).pathname.slice(1);
   if(['telemetry.js','updater-guard.js','memo-focus.js'].includes(name))return route.fulfill({body:'',contentType:'text/javascript'});
   const file=path.join(root,name);return route.fulfill({body:fs.readFileSync(file),contentType:/\.m?js$/.test(name)?'text/javascript':name.endsWith('.css')?'text/css':'text/html'});
  });
  await page.goto('http://jajan.test/'+(kind==='terminal'?'terminal.html':kind==='web'?'web.html':'index.html'));
  if(kind==='text')await page.waitForSelector('#memo-editor[aria-readonly]');
  if(kind==='image')await page.waitForSelector('#content img');
  if(kind==='terminal')await page.waitForSelector('#recommendations button');
  if(kind==='text'||kind==='image'){await page.locator('#copy').click();await page.waitForSelector('#feedback:not([hidden])');}
  if(kind==='terminal'){await page.locator('#pin').click();await page.locator('#choose-folder').click();}
  if(kind==='web'){await page.locator('#web-url').fill('https://example.com');await page.locator('#run').click();await page.waitForFunction(()=>document.querySelector('#status').textContent.includes('연결 실패'));}
  for(const [i,palette] of colorPalette.entries()) {
   await page.evaluate(color=>{window.note.color=color;window.listeners['memo-color']?.({payload:color});},palette.background);
   await page.waitForFunction(bg=>getComputedStyle(document.documentElement).getPropertyValue('--background').trim()===bg,palette.background);
   const selectors=kind==='text'?['#memo-editor','#pin','#close','#message']:kind==='image'?['#pin','#close','#message']:kind==='terminal'?['#choose-folder','#home-hint','#run-task','#pin','#close','#status']:['#web-url','#run','#pin','#close','#status'];
   const inspect=async()=>page.evaluate(selectors=>{
    const hex=rgb=>'#'+rgb.match(/[\d.]+/g).slice(0,3).map(v=>Number(v).toString(16).padStart(2,'0')).join('');
    function bg(el){const c=getComputedStyle(el).backgroundColor;return c==='rgba(0, 0, 0, 0)'&&el.parentElement?bg(el.parentElement):hex(c);}
    return selectors.map(selector=>{const el=document.querySelector(selector),s=getComputedStyle(el);return{selector,foreground:hex(s.color),background:bg(el),opacity:s.opacity};});
   },selectors);
   for(const [width,height] of [[240,160],[600,400],[320,300]]) {
   await page.setViewportSize({width,height});
   assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
   for(const state of ['normal','hover','focus','disabled']) {
    const button=page.locator('#close');
    if(state==='hover')await button.hover();
    if(state==='focus'){await page.keyboard.press('Tab');await button.focus();}
    if(state==='disabled')await button.evaluate(el=>el.disabled=true);
    const roles=await inspect();for(const role of roles){const ratio=contrastRatio(role.foreground,role.background);assert(ratio>=4.5,JSON.stringify({kind,palette:palette.name,state,...role,ratio}));assert.equal(role.opacity,'1');}
    results.push({kind,color:palette.background,width,height,state,roles});
    if(state==='disabled')await button.evaluate(el=>el.disabled=false);
   }
   }
   if(kind==='terminal')assert.deepEqual(await page.evaluate(()=>terminal.options.theme),{background:'#20252e',foreground:'#f0f0f0'});
   await page.screenshot({path:`${out}/${kind}-${i}.png`});
  }
  await page.emulateMedia({forcedColors:'active'});await page.keyboard.press('Tab');await page.locator('#close').focus();
  const forced=await page.evaluate(()=>{const button=document.querySelector('#close');const span=document.createElement('span');span.style.color='Highlight';span.style.forcedColorAdjust='none';document.body.append(span);const expected=getComputedStyle(span).color;span.remove();return {actual:getComputedStyle(button).outlineColor,expected,visible:button.matches(':focus-visible')};});assert.equal(forced.actual,forced.expected,JSON.stringify({kind,...forced}));
  await page.close();
 }
 assert.deepEqual(errors,[]);fs.writeFileSync(`${out}/browser-results.json`,JSON.stringify(results,null,2));
 // Render the explanatory vector in Chromium, keeping Korean font shaping.
 const svg=await browser.newPage({viewport:{width:1038,height:625}});await svg.goto('file://'+path.resolve('docs/design/color-palette.svg'));await svg.screenshot({path:'docs/design/color-palette.png'});
 console.log(`PASS ${results.length} surface/color/state cases; app roles >=4.5; xterm palette preserved`);
}finally{await browser.close();}
