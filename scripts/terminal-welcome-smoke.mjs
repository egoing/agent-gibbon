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
  const scenario=new URL(location.href).searchParams.get('scenario');
  const info={id:'terminal-57002',kind:'terminal',title:'터미널',last_path:scenario==='long'?'/project/'+('긴 경로/'.repeat(80)):null};window.calls=[];
  const choices=scenario==='empty'?[]:[{id:'codex',label:'Codex',command:"'/long application path/bin/codex'"},{id:'claude',label:'Claude Code',command:"'/usr/local/bin/claude'"},{id:'git',label:'Git 로그',command:"'/usr/bin/git' log --oneline --decorate -20"},{id:'htop',label:'htop',command:"'/usr/bin/htop'"}];
  window.__TAURI__={window:{getCurrentWindow:()=>({startDragging:async()=>{},startResizeDragging:async()=>{}})},core:{Channel:class{},invoke:async(name,args)=>{window.calls.push({name,args});if(name==='terminal_welcome')return {info,choices};if(name==='terminal_start'&&scenario==='delayed-fail')return await new Promise((resolve,reject)=>{window.rejectStart=()=>reject(Error('지연 실행 실패 검증'));});if(name==='terminal_start'&&scenario==='fail')throw Error('실행 실패 검증');if(name==='terminal_folder')info.last_path='/tmp/project';if(name==='terminal_rename')info.title=args.title;return info;}}};
 });
 for(const [width,height] of [[720,460],[320,240]]){
  await page.setViewportSize({width,height});await page.goto('http://gibbon.test/terminal.html');
  await page.getByRole('heading',{name:'작업 시작'}).waitFor();
  assert.equal(await page.locator('#recommendations button').count(),4);
  assert.equal(await page.locator('#title-details').getAttribute('open'),null);
  fs.mkdirSync('/tmp/gibbon-welcome-review',{recursive:true});
  await page.screenshot({path:`/tmp/gibbon-welcome-review/welcome-${width}.png`});
  assert.equal(await page.locator('#work-path').isVisible(),false);
  assert.equal(await page.evaluate(()=>window.calls.some(c=>c.name==='terminal_start')),false);
  await page.getByRole('button',{name:'폴더 선택',exact:true}).click();
  assert.equal(await page.locator('#work-path').textContent(),'/tmp/project');
  await page.locator('#choose-folder').focus();
  for(let i=0;i<5;i++)await page.keyboard.press('Tab');
  assert.equal(await page.evaluate(()=>document.activeElement.parentElement.id),'command-details');
  await page.keyboard.press('Enter');
  assert.equal(await page.locator('#command-details').evaluate(el=>el.open),true);
  assert.match(await page.locator('#commands').textContent(),/git.*log/);
  await page.keyboard.press('Tab');
  assert.equal(await page.evaluate(()=>document.activeElement.parentElement.id),'title-details');
  await page.keyboard.press('Space');
  assert.equal(await page.locator('#title-details').evaluate(el=>el.open),true);await page.locator('#rename-title').fill('검증 제목');await page.getByRole('button',{name:'적용',exact:true}).click();
  assert.equal(await page.locator('#title').textContent(),'검증 제목');
  for(const id of ['choose-folder','open-shell']){const b=await page.locator('#'+id).boundingBox();assert(b.width>=60&&b.height>=32,JSON.stringify(b));}
  assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
  if(width===720)await page.screenshot({path:process.env.WELCOME_SCREENSHOT||'/tmp/agent-gibbon-is2-57/welcome-browser.png'});
  await page.getByRole('button',{name:'Git 로그',exact:true}).click();
  assert.equal(await page.locator('#welcome').isVisible(),false);assert.equal(await page.locator('#terminal').isVisible(),true);
  assert.equal(await page.evaluate(()=>window.calls.find(c=>c.name==='terminal_start').args.choice),'git');
  await page.getByRole('button',{name:'터미널 종료',exact:true}).click();assert.equal(await page.evaluate(()=>window.calls.at(-1).name),'terminal_close');
  for(const scenario of ['empty','long','fail','delayed-fail']){
   await page.goto('http://gibbon.test/terminal.html?scenario='+scenario);
   await page.getByRole('heading',{name:'작업 시작'}).waitFor();
   await page.waitForFunction(()=>window.calls.some(c=>c.name==='terminal_welcome'));
   assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth && document.getElementById('welcome').scrollWidth<=document.getElementById('welcome').clientWidth));
   const shell=await page.locator('#open-shell').boundingBox();assert(shell.y>=0&&shell.y+shell.height<=height);
   if(scenario==='empty'){
    assert.equal(await page.locator('#recommendations button').count(),0);
    assert.equal(await page.locator('#no-apps').isVisible(),true);
    assert.equal(await page.locator('#command-details').isVisible(),false);
    await page.locator('#open-shell').click();
    assert.equal(await page.evaluate(()=>window.calls.find(c=>c.name==='terminal_start').args.choice),'shell');
   }else if(scenario==='long'){
    assert.equal(await page.locator('#work-path').textContent(),'/project/'+('긴 경로/'.repeat(80)));
    assert.equal(await page.locator('#home-hint').isVisible(),false);
    assert(await page.locator('#work-path').evaluate(el=>el.scrollHeight>el.clientHeight));
    await page.locator('#work-path').focus();await page.keyboard.press('Tab');
    assert.equal(await page.evaluate(()=>document.activeElement.id),'choose-folder');
   }else if(scenario==='delayed-fail'){
    await page.getByRole('button',{name:'Codex',exact:true}).click();
    await page.waitForFunction(()=>typeof window.rejectStart==='function');
    await page.locator('#pin').focus();
    await page.evaluate(()=>window.rejectStart());
    await page.waitForFunction(()=>document.getElementById('status').textContent.includes('지연 실행 실패 검증'));
    assert.equal(await page.evaluate(()=>document.activeElement.id),'pin');
    assert.equal(await page.locator('#welcome').isVisible(),true);
   }else{
    await page.getByRole('button',{name:'Codex',exact:true}).click();
    await page.waitForFunction(()=>document.getElementById('status').textContent.includes('실행 실패 검증'));
    assert.equal(await page.evaluate(()=>document.activeElement.getAttribute('aria-label')),'Codex');
    assert.equal(await page.locator('#welcome').isVisible(),true);
    assert.equal(await page.locator('#welcome').getAttribute('aria-busy'),'false');
    await page.locator('#open-shell').click();
    assert.equal(await page.evaluate(()=>document.activeElement.id),'open-shell');
   }
  }
 }
 assert.deepEqual(errors,[]);console.log('PASS: real Welcome assets at 720×460 and 320×240, four/zero apps, long path, keyboard details, failed launch focus, folder, rename, app/shell launch, close dispatch');
}finally{await browser.close();}
