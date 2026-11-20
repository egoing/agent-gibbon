// Real manager assets, mocked IPC. Never changes OS startup or consent.
import fs from 'node:fs';
import assert from 'node:assert/strict';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { join } from 'node:path';
const {default:puppeteer}=await import(process.env.PUPPETEER_MODULE || 'puppeteer-core');
const root=fileURLToPath(new URL('..',import.meta.url));
const out=process.env.AUTOSTART_OUTPUT;
assert.ok(out,'AUTOSTART_OUTPUT must name the verification artifact directory');
fs.mkdirSync(out,{recursive:true});
const browser=await puppeteer.launch({executablePath:process.env.CHROME_BIN,headless:true});
try {
  // Render maintained SVGs with the same browser used for layout inspection.
  const render=await browser.newPage();
  for(const name of ['privacy-onboarding','manager-settings-wireframe','login-autostart','updater-settings']) {
    const path=join(root,'docs/design',`${name}.svg`);
    const svg=fs.readFileSync(path,'utf8');
    const [,width,height]=svg.match(/<svg[^>]*width="(\d+)" height="(\d+)"/);
    await render.setViewport({width:+width,height:+height,deviceScaleFactor:1});
    await render.goto(pathToFileURL(path).href);
    await render.screenshot({path:join(root,'docs/design',`${name}.png`)});
  }
  await render.close();
  const page=await browser.newPage(),errors=[];
  page.on('pageerror',e=>errors.push(e.message));
  await page.setRequestInterception(true);
  page.on('request',r=>{
    const url=new URL(r.url());if(url.hostname!=='jajan.test')return r.abort();
    const file=join(root,'ui',url.pathname);
    if(!fs.existsSync(file))return r.respond({status:404,body:''});
    r.respond({status:200,contentType:/\.(m?js)$/.test(file)?'text/javascript':file.endsWith('.css')?'text/css':'text/html',body:fs.readFileSync(file)});
  });
  await page.evaluateOnNewDocument(()=>{
    window.calls=[];window.privacyAccepted=false;window.autoState={selected:null,available:true,os_status:'disabled'};
    window.__TAURI__={core:{invoke:async(name,args)=>{
      window.calls.push({name,args});
      if(name==='autostart_status')return {...window.autoState};
      if(name==='autostart_save'){window.autoState={...window.autoState,selected:args.enabled,os_status:args.enabled?'registered_unknown':'disabled'};return {...window.autoState};}
      if(name==='autostart_open_settings')return;
      if(name==='privacy_status')return {terms_accepted:window.privacyAccepted,sharing:false,allowed:false,notice_ready:true,terms_html:'<h1>Terms</h1>',notice_html:'<h1>Notice</h1>'};
      if(name==='privacy_save'){window.privacyAccepted=true;return {terms_accepted:true,sharing:args.shareDiagnostics,notice_ready:true};}
      if(name==='manager_notes')return [];
      if(name==='manager_set_theme')return;
      if(name==='skills_list')return {skills:[],feedback:null};
      if(name==='telemetry_status')return {detailed:false};
      if(name==='updater_status')return {phase:'disabled',current_version:'0.3.5'};
      throw Error('Unexpected IPC '+name);
    }}};
  });
  await page.setViewport({width:944,height:720});
  await page.goto('http://jajan.test/manager.html');
  await page.waitForFunction(()=>!document.getElementById('autostart-welcome').disabled);
  assert.equal(await page.$eval('#autostart-welcome',e=>e.checked),true);
  assert.equal(await page.$eval('#privacy-share',e=>e.checked),false);
  assert.equal(await page.evaluate(()=>window.calls.some(c=>c.name==='autostart_save')),false);
  await page.$eval('#privacy-welcome',e=>{e.style.flex='none';e.style.overflow='visible';});
  await page.screenshot({path:join(out,'onboarding.png'),fullPage:true});
  await page.$eval('#privacy-welcome',e=>{e.style.removeProperty('flex');e.style.removeProperty('overflow');});
  await page.click('#privacy-terms');await page.click('#privacy-continue');
  await page.waitForFunction(()=>!document.getElementById('manager-navigation').hidden);
  await page.click('#settings-nav');await page.click('#settings-autostart-nav');
  await page.waitForFunction(()=>document.getElementById('autostart-status').textContent.includes('허용 여부 미확인'));
  assert.equal(await page.evaluate(()=>window.calls.filter(c=>c.name==='autostart_save').length),1);
  assert.equal(await page.$eval('#autostart-os-settings',e=>e.hidden),false);
  await page.screenshot({path:join(out,'settings-windows.png')});
  for(const state of [
    {selected:true,os_status:'requires_approval',available:true},
    {selected:false,os_status:'disabled',available:true},
    {selected:null,os_status:'unavailable',available:false,reason:'개발·검증 앱은 로그인 자동 실행에 등록하지 않습니다.'},
  ]) {
    await page.evaluate(async s=>{window.autoState=s;await window.jajanAutostart.refresh();},state);
    await page.waitForFunction((selected)=>document.getElementById('autostart-choice').textContent.includes(selected===null?'아직':selected?'켜기':'끄기'),{},state.selected);
    assert.equal(await page.evaluate(()=>window.calls.filter(c=>c.name==='autostart_save').length),1);
    assert.equal(await page.$eval('#autostart-os-settings',e=>e.hidden),!['requires_approval','registered_unknown'].includes(state.os_status));
  }
  await page.setViewport({width:600,height:360});
  await Promise.all([page.waitForNavigation(),page.evaluate(()=>{localStorage.setItem('jajan.manager-theme','black');location.reload();})]);
  await page.waitForFunction(()=>!document.getElementById('privacy-terms').disabled);
  await page.click('#privacy-terms');await page.click('#privacy-continue');
  await page.waitForFunction(()=>!document.getElementById('manager-navigation').hidden);
  await page.click('#settings-nav');await page.click('#settings-autostart-nav');
  assert.equal(await page.$eval('#settings-autostart',e=>e.scrollWidth<=e.clientWidth),true);
  await page.focus('#autostart-setting');await page.keyboard.press('Space');
  assert.equal(await page.$eval('#autostart-setting',e=>e.checked),false);
  await page.waitForFunction(()=>document.getElementById('autostart-choice').textContent.includes('끄기'));
  await page.screenshot({path:join(out,'settings-minimum-dark.png')});
  assert.deepEqual(errors,[]);
  console.log(JSON.stringify({ok:true,svg_rendered:4,checks:['default-on proposal without registration','separate diagnostics','explicit Continue','Windows approval unknown','OS disabled without reenable','existing Off','development unavailable','conditional OS settings','600x360 layout','keyboard Off saves immediately'],output:out}));
} finally { await browser.close(); }
