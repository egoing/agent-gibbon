// Real assets in Chromium with native Tauri calls mocked; no native PTY claim.
// Approved wireframe changes app-click launch to select + Run and replaces
// all-command details with a persistent preview of the selected folder/task.
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
  const folder=path=>({path,name:path.split('/').at(-1),parent:path.slice(0,path.lastIndexOf('/')),command:`cd -- '${path}'`});
  let folders=scenario==='home'||scenario==='empty'?[]:['/Users/demo/project/website','/Users/demo/project/api-server','/Users/demo/sandbox','/Users/demo/project/fourth'].map(folder);
  if(scenario==='long')folders=[folder('/project/'+('긴 경로/'.repeat(80))+'끝'),...folders];
  const info={id:'terminal-57002',kind:'terminal',title:'터미널',last_path:folders[0]?.path||null};window.calls=[];
  const choices=scenario==='empty'?[]:[{id:'claude',label:'Claude Code',command:"'/usr/local/bin/claude'"},{id:'codex',label:'Codex',command:"'/long application path/bin/codex'"},{id:'git',label:'git log',command:"'/usr/bin/git' log --oneline --decorate -20"},{id:'htop',label:'htop',command:"'/usr/bin/htop'"}];
  choices.push({id:'shell',label:'셸',command:"'/bin/zsh' -i"});
  window.__TAURI__={window:{getCurrentWindow:()=>({startDragging:async()=>{},startResizeDragging:async()=>{}})},core:{Channel:class{},invoke:async(name,args)=>{
   window.calls.push({name,args});
   if(name==='terminal_welcome')return {info,choices,folders,home:'/Users/demo',location:folder(info.last_path||'/Users/demo')};
   if(name==='terminal_start'&&scenario==='delayed-fail')return await new Promise((resolve,reject)=>{window.rejectStart=()=>reject(Error('지연 실행 실패 검증'));});
   if(name==='terminal_start'&&scenario==='fail')throw Error('실행 실패 검증');
   if(name==='terminal_folder'&&args?.path===null&&scenario==='cancel')return await new Promise(resolve=>{window.cancelPicker=()=>resolve({folder_selected:false});});
   if(name==='terminal_folder')info.last_path=args?.path||'/tmp/chosen project';
   if(name==='terminal_rename')info.title=args.title;
   return info;
  }}};
 });
 const noStart=async()=>assert.equal(await page.evaluate(()=>window.calls.some(c=>c.name==='terminal_start')),false);
 const noOverflow=async()=>assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth&&document.getElementById('welcome').scrollWidth<=document.getElementById('welcome').clientWidth));
 const open=async scenario=>{await page.goto('http://gibbon.test/terminal.html?scenario='+scenario);await page.getByRole('heading',{name:'작업 위치'}).waitFor();await page.waitForFunction(()=>document.querySelectorAll('#recommendations button').length>0);};
 for(const [width,height] of [[720,460],[320,240]]){
  await page.setViewportSize({width,height});await open('recent');
  assert.equal(await page.locator('#recent-folders button').count(),3);
  assert.equal(await page.locator('#recommendations button').count(),5);
  assert.equal(await page.locator('#title-details').getAttribute('open'),null);
  await noStart();await noOverflow();
  fs.mkdirSync('/tmp/gibbon-wireframe',{recursive:true});await page.screenshot({path:`/tmp/gibbon-wireframe/welcome-${width}.png`});
  await page.locator('#choose-folder').focus();await page.keyboard.press('Tab');
  assert.equal(await page.evaluate(()=>document.activeElement.dataset.path),'/Users/demo/project/website');
  await page.keyboard.press('Tab');await page.keyboard.press('Space');
  assert.equal(await page.locator('#recent-folders button').nth(1).getAttribute('aria-pressed'),'true');
  assert.match(await page.locator('#command-preview').textContent(),/api-server/);
  assert.equal(await page.evaluate(()=>window.calls.some(c=>c.name==='terminal_folder')),false);
  const colors=await page.locator('#recent-folders button').evaluateAll(nodes=>nodes.map(n=>getComputedStyle(n).backgroundColor));assert.notEqual(colors[0],colors[1]);
  await page.locator('#more-folders').click();assert.equal(await page.locator('#recent-folders button').count(),4);
  await page.locator('#recent-folders button').nth(3).click();assert.match(await page.locator('#command-preview').textContent(),/fourth/);
  await page.getByRole('button',{name:'Codex',exact:true}).click();await noStart();
  assert.equal(await page.getByRole('button',{name:'Codex',exact:true}).getAttribute('aria-pressed'),'true');
  assert.match(await page.locator('#command-preview').textContent(),/'\/long application path\/bin\/codex'/);
  await page.locator('#more-folders').click();assert.equal(await page.locator('#recent-folders button').count(),3);
  // Every item stays keyboard reachable even when the small viewport scrolls.
  await page.getByRole('button',{name:'셸',exact:true}).focus();await page.keyboard.press('Tab');
  assert.equal(await page.evaluate(()=>document.activeElement.id),'run-task');
  const runBox=await page.locator('#run-task').boundingBox();assert(runBox.y>=0&&runBox.y+runBox.height<=height);
  if(width===320)assert(await page.locator('#welcome').evaluate(el=>el.scrollHeight>el.clientHeight));
  await page.keyboard.press('Tab');assert.equal(await page.evaluate(()=>document.activeElement.id),'command-preview');
  await page.keyboard.press('Tab');await page.keyboard.press('Enter');assert.equal(await page.locator('#title-details').evaluate(el=>el.open),true);
  await page.locator('#rename-title').fill('검증 제목');await page.getByRole('button',{name:'적용',exact:true}).click();assert.equal(await page.locator('#title').textContent(),'검증 제목');
  await page.locator('#run-task').click();
  assert.equal(await page.locator('#welcome').isVisible(),false);
  const writes=await page.evaluate(()=>window.calls.filter(c=>c.name==='terminal_folder'||c.name==='terminal_start'));assert.equal(writes[0].args.path,'/Users/demo/project/fourth');assert.equal(writes[1].args.choice,'codex');
  await page.getByRole('button',{name:'터미널 종료',exact:true}).click();assert.equal(await page.evaluate(()=>window.calls.at(-1).name),'terminal_close');
  for(const scenario of ['home','empty','long','cancel','fail','delayed-fail']){
   await open(scenario);await noStart();await noOverflow();
   assert.equal(await page.getByRole('button',{name:'셸',exact:true}).count(),1);
   if(scenario==='home'||scenario==='empty'){
    assert.equal(await page.locator('#recent-folders button').count(),0);
    assert.equal(await page.locator('#home-hint').isVisible(),true);assert.match(await page.locator('#command-preview').textContent(),/cd -- '\/Users\/demo'/);
    if(scenario==='home'){
     await page.locator('#choose-folder').click();assert.match(await page.locator('#command-preview').textContent(),/chosen project/);await noStart();
    }else{
     assert.equal(await page.locator('#recommendations button').count(),1);await page.getByRole('button',{name:'셸',exact:true}).click();await noStart();await page.locator('#run-task').click();assert.equal(await page.evaluate(()=>window.calls.find(c=>c.name==='terminal_start').args.choice),'shell');
    }
   }else if(scenario==='cancel'){
    await page.locator('#recent-folders button').nth(1).click();
    const preview=await page.locator('#command-preview').textContent();
    await page.locator('#choose-folder').click();await page.waitForFunction(()=>typeof window.cancelPicker==='function');
    assert.equal(await page.locator('#run-task').isDisabled(),true);await noStart();
    await page.evaluate(()=>window.cancelPicker());await page.waitForFunction(()=>!document.getElementById('run-task').disabled);
    assert.equal(await page.locator('#recent-folders button').nth(1).getAttribute('aria-pressed'),'true');
    assert.equal(await page.locator('#command-preview').textContent(),preview);
    assert.equal(await page.evaluate(()=>window.calls.filter(c=>c.name==='terminal_welcome').length),1);await noStart();
   }else if(scenario==='long'){
    const longPath='/project/'+('긴 경로/'.repeat(80))+'끝';
    assert.equal(await page.locator('#recent-folders button').first().getAttribute('title'),longPath);
    assert.ok((await page.locator('#command-preview').textContent()).includes(longPath));
    await page.locator('#run-task').focus();const box=await page.locator('#run-task').boundingBox();assert(box.y>=0&&box.y+box.height<=height);await noOverflow();
   }else{
    await page.locator('#run-task').click();
    if(scenario==='delayed-fail'){await page.waitForFunction(()=>typeof window.rejectStart==='function');await page.locator('#pin').focus();await page.evaluate(()=>window.rejectStart());}
    await page.waitForFunction(()=>document.getElementById('status').textContent.includes('실행 실패 검증'));
    assert.equal(await page.evaluate(()=>document.activeElement.id),scenario==='delayed-fail'?'pin':'run-task');
    assert.equal(await page.locator('#welcome').isVisible(),true);assert.equal(await page.locator('#welcome').getAttribute('aria-busy'),'false');
   }
  }
 }
 await page.setViewportSize({width:720,height:900});await open('recent');await noOverflow();
 const centered=await page.evaluate(()=>{const welcome=document.getElementById('welcome'),content=document.querySelector('.welcome-content');const outer=welcome.getBoundingClientRect(),inner=content.getBoundingClientRect();return {gapTop:inner.top-outer.top,gapBottom:outer.bottom-inner.bottom,scroll:welcome.scrollHeight>welcome.clientHeight};});
 assert.equal(centered.scroll,false);assert(centered.gapTop>24);assert(Math.abs(centered.gapTop-centered.gapBottom)<=2,JSON.stringify(centered));
 const rowStyle=await page.getByRole('button',{name:'Claude Code',exact:true}).evaluate(el=>({align:getComputedStyle(el).justifyContent,weight:getComputedStyle(el).fontWeight}));
 assert.deepEqual(rowStyle,{align:'flex-start',weight:'400'});
 await page.screenshot({path:'/tmp/gibbon-wireframe/welcome-tall.png'});
 assert.deepEqual(errors,[]);console.log('PASS: approved wireframe at 720×460 / 320×240; recent 3/more, selection + preview without start, Run, shell/home, keyboard/scroll/long path, delayed failure focus');
}finally{await browser.close();}
