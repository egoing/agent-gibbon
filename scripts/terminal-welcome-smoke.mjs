// Real WebView assets under Chromium; native PTY/dialog lifecycle is tested separately.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
const {chromium}=await import(process.env.PLAYWRIGHT_MODULE||'playwright');
const root=fileURLToPath(new URL('..',import.meta.url));
const browser=await chromium.launch({headless:true});
try{
 const page=await browser.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.route('http://gibbon.test/**',route=>{const f=path.join(root,'ui',new URL(route.request().url()).pathname);return route.fulfill({body:fs.readFileSync(f),contentType:f.endsWith('.css')?'text/css':f.endsWith('.js')?'text/javascript':'text/html'});});
 await page.addInitScript(()=>{
  const info={id:'terminal-57002',kind:'terminal',title:'터미널',last_path:null};window.calls=[];
  window.__TAURI__={window:{getCurrentWindow:()=>({startDragging:async()=>{},startResizeDragging:async()=>{}})},core:{Channel:class{},invoke:async(name,args)=>{window.calls.push({name,args});if(name==='terminal_welcome')return {info,choices:[{id:'git',label:'Git 로그',command:"'/usr/bin/git' log --oneline --decorate -20"}]};if(name==='terminal_folder')info.last_path='/tmp/project';if(name==='terminal_rename')info.title=args.title;return info;}}};
 });
 for(const [width,height] of [[720,460],[320,240]]){
  await page.setViewportSize({width,height});await page.goto('http://gibbon.test/terminal.html');
  await page.getByRole('heading',{name:'Welcome'}).waitFor();
  assert.equal(await page.locator('#work-path').isVisible(),false);
  assert.equal(await page.evaluate(()=>window.calls.some(c=>c.name==='terminal_start')),false);
  await page.getByRole('button',{name:'폴더 선택',exact:true}).click();
  assert.equal(await page.locator('#work-path').textContent(),'/tmp/project');
  await page.getByText('실행 명령 확인',{exact:true}).click();
  assert.match(await page.locator('#commands').textContent(),/git.*log/);
  await page.locator('#rename-title').fill('검증 제목');await page.getByRole('button',{name:'제목 변경',exact:true}).click();
  assert.equal(await page.locator('#title').textContent(),'검증 제목');
  for(const id of ['choose-folder','open-shell']){const b=await page.locator('#'+id).boundingBox();assert(b.width>=60&&b.height>=32,JSON.stringify(b));}
  assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
  if(width===720)await page.screenshot({path:process.env.WELCOME_SCREENSHOT||'/tmp/agent-gibbon-is2-57/welcome-browser.png'});
  await page.getByRole('button',{name:'Git 로그',exact:true}).click();
  assert.equal(await page.locator('#welcome').isVisible(),false);assert.equal(await page.locator('#terminal').isVisible(),true);
  assert.equal(await page.evaluate(()=>window.calls.find(c=>c.name==='terminal_start').args.choice),'git');
  await page.getByRole('button',{name:'터미널 종료',exact:true}).click();assert.equal(await page.evaluate(()=>window.calls.at(-1).name),'terminal_close');
 }
 assert.deepEqual(errors,[]);console.log('PASS: real Welcome assets at 720×460 and 320×240, no-path, folder, command, rename, app launch, close dispatch');
}finally{await browser.close();}
