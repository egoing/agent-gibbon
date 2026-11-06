// Actual manager assets in Chromium. IPC is mocked; no consent is saved.
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
const {default:puppeteer}=await import(process.env.PUPPETEER_MODULE || 'puppeteer-core');
const root=fileURLToPath(new URL('..',import.meta.url));
const out=process.env.THEME_OUTPUT || '/tmp/gibbon-theme';fs.mkdirSync(out,{recursive:true});
const browser=await puppeteer.launch({executablePath:process.env.CHROME_BIN || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
const contrast=(a,b)=>{
 const lum=rgb=>rgb.map(c=>c/255).map(c=>c<=.04045?c/12.92:((c+.055)/1.055)**2.4).reduce((n,c,i)=>n+c*[.2126,.7152,.0722][i],0);
 const x=lum(a),y=lum(b);return (Math.max(x,y)+.05)/(Math.min(x,y)+.05);
};
const rgb=hex=>hex.slice(1).match(/../g).map(x=>parseInt(x,16));
const results=[];
try {
 const page=await browser.newPage(),errors=[],external=[];page.on('pageerror',e=>errors.push(e.message));
 await page.setRequestInterception(true);
 page.on('request',r=>{const u=new URL(r.url());if(u.hostname!=='gibbon.test'){external.push(r.url());return r.abort();}const f=root+'ui'+u.pathname;if(!fs.existsSync(f))return r.respond({status:404,body:''});r.respond({status:200,contentType:/\.(m?js)$/.test(f)?'text/javascript':f.endsWith('.css')?'text/css':'text/html',body:fs.readFileSync(f)});});
 await page.evaluateOnNewDocument(()=>{
  window.calls=[];window.onboarding=false;
  window.__TAURI__={core:{invoke:async(name,args)=>{window.calls.push({name,args});
   if(name==='privacy_status')return {terms_accepted:false,terms_skipped:!window.onboarding,allowed:false,sharing:false,notice_ready:true,terms_html:'<h1>Terms of Use</h1><p>로컬 문서 읽기</p><a href="#">문서 링크</a>',notice_html:'<h1>Privacy Notice</h1>'};
   if(name==='skills_list')return {skills:[{id:'a',name:'메모 관리',description:'기존 메모 상태를 유지합니다.',enabled:false},{id:'b',name:'작업 진행 표시',description:'활성 상태 예시',enabled:true}],feedback:null};
   if(name==='manager_notes')return [{id:'theme-probe',title:'테마 검증 메모',html:'<h1>테마 검증 메모</h1><p>본문과 <a href="#">링크</a>, 표를 읽습니다.</p><table><tr><th>항목</th><td>검증</td></tr></table>',visible:false}];
   if(name==='manager_set_theme'){if(!['white','gray','black'].includes(args.theme))throw Error('Invalid native theme');return;}
   if(name==='telemetry_status')return {detailed:false};
   throw Error('Unexpected IPC '+name);
  }}};
 });
 const start=async()=>{await page.goto('http://gibbon.test/manager.html');await page.waitForFunction(()=>!document.getElementById('manager-navigation').hidden);};
 await page.emulateMediaFeatures([{name:'prefers-color-scheme',value:'light'}]);await start();
 assert.equal(await page.$eval('html',e=>e.dataset.managerTheme),'gray');
 await page.emulateMediaFeatures([{name:'prefers-color-scheme',value:'dark'}]);await page.waitForFunction(()=>document.documentElement.dataset.managerTheme==='black');assert.equal(await page.$eval('html',e=>e.dataset.managerTheme),'black');
 assert.equal(await page.evaluate(()=>localStorage.getItem('agent-gibbon.manager-theme')),null);
 for(const [width,height] of [[944,480],[600,360]]) {
  await page.setViewport({width,height});
  for(const theme of ['white','gray','black']) {
   await page.click('#settings-nav');await page.click('#settings-appearance-nav');
   await page.focus(`input[value="${theme}"]`);await page.keyboard.press('Space');
   assert.equal(await page.$eval('html',e=>e.dataset.managerTheme),theme);
   assert.equal(await page.evaluate(()=>document.activeElement.value),theme);
   assert.equal(await page.evaluate(()=>localStorage.getItem('agent-gibbon.manager-theme')),theme);
   await page.emulateMediaFeatures([{name:'prefers-color-scheme',value:theme==='black'?'light':'dark'}]);assert.equal(await page.$eval('html',e=>e.dataset.managerTheme),theme);
   assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
   assert(await page.$eval('.settings-content',e=>e.scrollWidth<=e.clientWidth));
   const tokens=await page.$eval('html',e=>{const s=getComputedStyle(e);return Object.fromEntries(['bg','text','muted','focus-surface','modal','error','error-surface','success','success-surface','focus','disabled','control-border'].map(k=>[k,s.getPropertyValue('--manager-'+k).trim()]));});
   const ratios={};for(const name of ['text','muted','disabled','focus']) for(const surface of ['bg','focus-surface','modal']) {const ratio=contrast(rgb(tokens[name]),rgb(tokens[surface]));ratios[name+'/'+surface]=ratio;assert(ratio>=4.5,`${theme} ${name}/${surface}: ${ratio}`);}
   for(const name of ['error','success']){const ratio=contrast(rgb(tokens[name]),rgb(tokens[name+'-surface']));ratios[name]=ratio;assert(ratio>=4.5);}
   for(const surface of ['bg','focus-surface'])assert(contrast(rgb(tokens['control-border']),rgb(tokens[surface]))>=3,theme+' control boundary');
   results.push({width,theme,tokens,ratios});await page.screenshot({path:`${out}/settings-${theme}-${width}.png`});
   await start();assert.equal(await page.$eval('html',e=>e.dataset.managerTheme),theme);
   await page.click('#list button');await page.screenshot({path:`${out}/notes-${theme}-${width}.png`});
   await page.click('#settings-nav');await page.click('#settings-skills-nav');await page.waitForSelector('.skill-item');await page.click('.skill-item summary');await page.screenshot({path:`${out}/skills-${theme}-${width}.png`});
   await page.click('#settings-privacy-nav');await page.click('#privacy-settings-terms');await page.screenshot({path:`${out}/document-${theme}-${width}.png`});await page.click('#privacy-document-close');
   await page.evaluate(async()=>{window.onboarding=true;await window.gibbonPrivacy.refresh();});
   assert.equal(await page.$eval('#privacy-terms',e=>e.checked),false);assert.equal(await page.$eval('#privacy-share',e=>e.checked),false);
   assert.equal(await page.$eval('#privacy-continue',e=>e.disabled),true);
   await page.screenshot({path:`${out}/welcome-${theme}-${width}.png`});await start();
  }
 }
 await page.click('#settings-nav');await page.click('#settings-appearance-nav');await page.click('input[name="manager-theme"][value="system"]');
 assert.equal(await page.evaluate(()=>localStorage.getItem('agent-gibbon.manager-theme')),null);
 await page.emulateMediaFeatures([{name:'prefers-color-scheme',value:'dark'}]);await page.waitForFunction(()=>document.documentElement.dataset.managerTheme==='black');assert.equal(await page.$eval('html',e=>e.dataset.managerTheme),'black');
 await page.emulateMediaFeatures([{name:'prefers-color-scheme',value:'light'}]);await page.waitForFunction(()=>document.documentElement.dataset.managerTheme==='gray');assert.equal(await page.$eval('html',e=>e.dataset.managerTheme),'gray');
 // Selecting the current system color is an explicit choice too.
 await page.click('input[value=gray]');assert.equal(await page.evaluate(()=>localStorage.getItem('agent-gibbon.manager-theme')),'gray');
 await page.focus('input[value=gray]');await page.keyboard.press('ArrowRight');assert.equal(await page.$eval('html',e=>e.dataset.managerTheme),'black');
 assert.equal(await page.evaluate(()=>localStorage.getItem('agent-gibbon.manager-theme')),'black');
 await page.click('input[name="manager-theme"][value="system"]');
 // Real browser storage denial must keep the old selected radio and report failure.
 await page.evaluate(()=>{Storage.prototype.setItem=()=>{throw Error('blocked');};});
 await page.click('input[value=black]');assert.equal(await page.$eval('html',e=>e.dataset.managerTheme),'gray');assert.equal(await page.$eval('#theme-result',e=>e.dataset.error),'true');
 assert.equal(await page.evaluate(()=>window.calls.some(c=>c.name==='privacy_save'||c.name==='manager_set_visibility')),false);
 assert.deepEqual(errors,[]);assert.deepEqual(external,[]);
 fs.writeFileSync(`${out}/contrast.json`,JSON.stringify(results,null,2));
 console.log('PASS: 3 themes × 2 sizes × settings/notes/skills/document/welcome, keyboard focus, contrast, reload persistence, system changes/reset, storage denial, no consent/visibility writes or external requests.');
} finally {await browser.close();}
