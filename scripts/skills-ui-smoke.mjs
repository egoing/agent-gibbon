// Real manager assets in Chrome; native installer is verified separately.
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
const {default:puppeteer}=await import(process.env.PUPPETEER_MODULE||'puppeteer-core');
const root=fileURLToPath(new URL('..',import.meta.url));
const browser=await puppeteer.launch({executablePath:process.env.CHROME_BIN||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
try {
 const page=await browser.newPage();const errors=[],external=[];page.on('pageerror',e=>errors.push(e.message));
 await page.setRequestInterception(true);
 page.on('request',r=>{const url=new URL(r.url());if(url.hostname!=='gibbon.test'){external.push(r.url());return r.abort();}const file=root+'ui'+url.pathname;if(!fs.existsSync(file))return r.respond({status:404,body:''});return r.respond({status:200,contentType:file.endsWith('.css')?'text/css':file.endsWith('.js')?'text/javascript':'text/html',body:fs.readFileSync(file)});});
 await page.evaluateOnNewDocument(()=>{
  let skills=[{id:'agent-gibbon',name:'메모 관리',description:'Agent Gibbon의 메모 생성·조회·갱신·숨기기를 사용합니다. 기존 창의 위치와 포커스, 숨김 상태를 유지합니다.',enabled:false},{id:'agent-gibbon-progress',name:'작업 진행 표시',description:'작업 계획과 진행상황을 플로팅 체크리스트 메모로 표시합니다. 실제로 끝난 단계와 미검증 범위를 구분하며 같은 메모를 갱신합니다.',enabled:false}];
  window.calls=[];window.__TAURI__={core:{invoke:async(name,args)=>{
   window.calls.push({name,args});
   if(name==='privacy_status')return {terms_accepted:true,sharing:false,notice_ready:true};
   if(name==='manager_notes')return [];
   if(name==='skills_set'){
    if(window.delayInstall)await new Promise(resolve=>window.releaseInstall=resolve);
    if(window.failInstall)throw Error('설치 실패 검증');
    skills=skills.map(s=>s.id===args.id?{...s,enabled:args.enabled}:s);
   }
   if(name.startsWith('skills_'))return {skills:skills.map(s=>({...s})),feedback:null};
  }}};
 });
 const toggle=()=>page.$$('.skill-switch');
 for(const [width,height] of [[944,480],[600,360]]) {
  await page.setViewport({width,height});await page.goto('http://gibbon.test/manager.html');await page.waitForFunction(()=>!document.getElementById('manager-navigation').hidden);
  await page.click('#settings-nav');await page.click('#settings-skills-nav');await page.waitForSelector('.skill-item');
  assert.equal((await page.$$('.skill-item')).length,2);
  assert.equal(await page.$eval('#settings-skills',e=>e.querySelectorAll('input,select,textarea').length),0);
  assert.equal((await page.$$('#settings-skills button')).length,2);
  const copy=await page.$eval('#settings-skills',e=>e.textContent);assert(!/빌트인|스킬을 펼쳐|사용 예시|켜면 스킬을 덮어/.test(copy));
  const summary=(await page.$$('.skill-item summary'))[0];await summary.focus();await page.keyboard.press('Space');
  assert.equal(await page.$eval('.skill-item',e=>e.open),true);
  await page.keyboard.press('Tab');assert.equal(await page.evaluate(()=>document.activeElement.getAttribute('role')),'switch');
  await page.keyboard.press('Tab');assert.equal(await page.evaluate(()=>document.activeElement.tagName),'SUMMARY');
  await page.keyboard.down('Shift');await page.keyboard.press('Tab');await page.keyboard.up('Shift');assert.equal(await page.evaluate(()=>document.activeElement.getAttribute('role')),'switch');
  let buttons=await toggle();await page.evaluate(()=>window.delayInstall=true);await buttons[0].click();
  await page.waitForFunction(()=>Boolean(window.releaseInstall));assert.equal(await buttons[0].evaluate(e=>e.getAttribute('aria-checked')),'false');
  assert.equal(await buttons[0].evaluate(e=>e.getAttribute('aria-disabled')),'true');assert.equal(await buttons[1].evaluate(e=>e.getAttribute('aria-disabled')),'true');
  await page.evaluate(()=>{window.delayInstall=false;window.releaseInstall();});
  await page.waitForFunction(()=>document.querySelector('.skill-switch').getAttribute('aria-checked')==='true');
  assert.equal(await page.$eval('.skill-item',e=>e.dataset.enabled),'true');
  assert.equal(await page.evaluate(()=>document.activeElement.getAttribute('role')),'switch');
  assert.equal(await page.$eval('.skill-item',e=>getComputedStyle(e).backgroundColor),'rgb(238, 248, 239)');
  await summary.focus();await page.keyboard.press('Enter');assert.equal(await page.$eval('.skill-item',e=>e.open),false);
  assert.equal(await page.$eval('.skill-item',e=>e.dataset.enabled),'true');
  await summary.click();buttons=await toggle();await buttons[0].focus();await page.keyboard.press('Space');
  await page.waitForFunction(()=>document.querySelector('.skill-switch').getAttribute('aria-checked')==='false');
  await page.evaluate(()=>window.failInstall=true);await buttons[0].click();await page.waitForFunction(()=>!document.getElementById('skills-feedback').hidden);
  assert.equal(await buttons[0].evaluate(e=>e.getAttribute('aria-checked')),'false');
  await page.evaluate(()=>window.failInstall=false);await buttons[0].click();await page.waitForFunction(()=>document.querySelector('.skill-switch').getAttribute('aria-checked')==='true');
  const second=(await page.$$('.skill-item summary'))[1];await second.click();buttons=await toggle();await buttons[1].click();await page.waitForFunction(()=>document.querySelectorAll('.skill-switch')[1].getAttribute('aria-checked')==='true');
  await buttons[0].evaluate(e=>e.scrollIntoView());await buttons[0].click();await page.waitForFunction(()=>document.querySelector('.skill-switch').getAttribute('aria-checked')==='false');
  assert.equal(await buttons[1].evaluate(e=>e.getAttribute('aria-checked')),'true');
  const cdp=await page.createCDPSession();const {nodes}=await cdp.send('Accessibility.getFullAXTree');const switches=nodes.filter(n=>n.role?.value==='switch');
  assert.equal(switches.length,2);assert(switches.some(n=>n.name.value==='작업 진행 표시'&&n.properties.some(p=>p.name==='checked'&&p.value.value==='true')));await cdp.detach();
  assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));assert(await page.$eval('.settings-content',e=>e.scrollWidth<=e.clientWidth));
  await page.$eval('.settings-content',e=>e.scrollTop=0);fs.mkdirSync('/tmp/gibbon-skills-ui',{recursive:true});await page.screenshot({path:`/tmp/gibbon-skills-ui/expanded-${width}.png`});
  for(const s of await page.$$('.skill-item summary')){const open=await s.evaluate(e=>e.parentElement.open);if(open)await s.click();}
  await page.$eval('.settings-content',e=>e.scrollTop=0);await page.screenshot({path:`/tmp/gibbon-skills-ui/collapsed-${width}.png`});
 }
 assert.deepEqual(errors,[]);assert.deepEqual(external,[]);console.log('PASS: skills accordion, keyboard, accessible switch state, independent toggles, no optimistic install, error recovery, two viewports, no JS errors/external requests.');
}finally{await browser.close();}
