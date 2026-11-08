// Real UI assets in Chromium; update IPC is mocked, never installs an update.
import { readFileSync, existsSync, mkdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';
const {default:puppeteer}=await import(process.env.PUPPETEER_MODULE || 'puppeteer-core');
const root=fileURLToPath(new URL('..',import.meta.url)).replace(/\/$/,'');
const output=process.env.SCREENSHOT_DIR || '/tmp/gibbon-update-ui/browser';
mkdirSync(output,{recursive:true});
const browser=await puppeteer.launch({executablePath:process.env.CHROME_BIN || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
try {
 const page=await browser.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.setRequestInterception(true);
 page.on('request',r=>{const url=new URL(r.url());if(url.hostname!=='gibbon.test')return r.abort();const file=root+'/ui'+url.pathname;if(!existsSync(file))return r.respond({status:404,body:''});r.respond({status:200,contentType:/\.(m?js)$/.test(file)?'text/javascript':file.endsWith('.css')?'text/css':'text/html',body:readFileSync(file)});});
 await page.evaluateOnNewDocument(()=>{
  window.statusFixture={phase:'ready',version:'1.3.0',current_version:'1.2.3',message:'최근 입력이 있어 자동 설치를 보류했습니다. 5분 동안 조작하지 않으면 자동 설치합니다.'};
  window.requests=[]; window.__TAURI__={core:{invoke:async name=>{
   window.requests.push(name);
   if(name.startsWith('updater_'))return window.statusFixture;
   if(name==='privacy_status')return {terms_accepted:true,allowed:false,sharing:false,notice_ready:true};
   if(name==='manager_notes')return [];
   if(name==='skills_list')return {skills:[],feedback:null};
  }},event:{listen:async()=>()=>{}}};
 });
 for(const scheme of ['light','dark']){
  await page.emulateMediaFeatures([{name:'prefers-color-scheme',value:scheme}]);
  await page.setViewport({width:360,height:140});await page.goto('http://gibbon.test/updater-notice.html');
  await page.waitForFunction(()=>document.getElementById('notice-install').disabled===false);
  const bounds=await page.evaluate(()=>({scroll:document.documentElement.scrollHeight,height:innerHeight,width:document.documentElement.scrollWidth,buttons:[...document.querySelectorAll('button')].map(e=>({bottom:e.getBoundingClientRect().bottom,height:e.offsetHeight}))}));
  assert(bounds.scroll<=bounds.height,'normal notice must fit its compact 360×140 viewport');
  assert(bounds.width<=360,'notice must not overflow horizontally');
  assert(bounds.buttons.every(b=>b.bottom<=140&&b.height>=32),'actions remain visible and usable');
  await page.screenshot({path:`${output}/notice-${scheme}.png`});
  assert(await page.$eval('#notice-message',e=>e.hidden),'routine explanation is hidden');
  assert(!(await page.content()).includes('서명 검증 완료'));
  // Multiple blockers stay scrollable, and cancellation must not invoke the installer.
  await page.evaluate(()=>{window.statusFixture.review={version:'1.3.0',restores_sessions:true,targets:[
   {label:'webpage-web-test',kind:'web',title:'웹 창'},
   {label:'terminal-102',kind:'terminal',title:'빌드 터미널',pid:102}
  ]};return refreshNotice();});
  assert.equal((await page.$$('#notice-review button')).length,2);
  await page.click('#notice-review button');
  assert(await page.evaluate(()=>window.requests.includes('updater_show_target')));
  await page.click('#notice-install');
  assert.match(await page.$eval('#notice-review',e=>e.textContent),/프로세스는 종료/);
  assert(await page.$eval('#notice-dismiss',e=>e.textContent==='취소'&&e.getBoundingClientRect().bottom<=innerHeight));
  await page.screenshot({path:`${output}/notice-review-${scheme}.png`});
  await page.click('#notice-dismiss');
  assert(!await page.evaluate(()=>window.requests.includes('updater_notice_install')));
  // Errors remain readable in the scroll area; actions stay on screen.
  await page.evaluate(()=>{window.statusFixture={...window.statusFixture,phase:'error',version:'1.3.0-'+'long'.repeat(30),message:'작업 상태를 확인한 뒤 다시 시도하세요. '.repeat(20)};return refreshNotice();});
  assert(await page.$eval('#notice-message',e=>!e.hidden&&e.textContent.length>200));
  assert(await page.$eval('#notice-install',e=>e.getBoundingClientRect().bottom<=innerHeight),'long detail must not push actions off screen');
  assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
 }
 for(const [width,height] of [[944,480],[600,360]]){
  await page.setViewport({width,height});await page.goto('http://gibbon.test/manager.html');
  await page.waitForFunction(()=>!document.getElementById('manager-navigation').hidden);
  await page.click('#settings-nav');await page.click('#settings-updater-nav');
  for(const theme of ['white','gray','black']){
   await page.evaluate(theme=>document.documentElement.dataset.managerTheme=theme,theme);
   for(const phase of ['idle','checking','downloading','ready','preparing','installing','error','disabled']){
    await page.evaluate(phase=>{window.statusFixture={phase,version:['ready','preparing','installing','downloading'].includes(phase)?'1.3.0':null,current_version:'1.2.3',downloaded:40,total:100,message:phase==='idle'?'':phase==='error'?'서명 검증 실패: 다시 시도하세요.':phase==='disabled'?'개발용 빌드에서는 자동 업데이트가 꺼져 있습니다.':'작업 상태를 확인한 뒤 설치합니다.'};return window.gibbonUpdater.refresh();},phase);
    assert(await page.$eval('.settings-content',e=>e.scrollWidth<=e.clientWidth),'settings must not overflow horizontally');
    const installVisible=await page.$eval('#updater-install',e=>!e.hidden);
    assert.equal(installVisible,['ready','preparing','installing'].includes(phase));
    if(phase==='ready'){
     const card=await page.$eval('.updater-card',e=>e.getBoundingClientRect().height);
     assert(card<=190,'settings status area must stay compact');
     await page.screenshot({path:`${output}/settings-${width}-${theme}.png`});
    }
   }
  }
 }
 assert.deepEqual(errors,[]); console.log('PASS: compact notice light/dark + long content, eight settings states × three themes × two sizes; real assets, mocked IPC');
} finally { await browser.close(); }
