// Browser verification of real manager assets; Tauri storage calls are mocked.
const {default:puppeteer}=await import(process.env.PUPPETEER_MODULE || 'puppeteer-core');
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
const root=fileURLToPath(new URL('..',import.meta.url)).replace(/\/$/,'');
const browser=await puppeteer.launch({executablePath:process.env.CHROME_BIN || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
try {
const page=await browser.newPage(); const errors=[];page.on('pageerror',e=>errors.push(e.message));
await page.setRequestInterception(true);
page.on('request',r=>{const url=new URL(r.url());if(url.hostname!=='gibbon.test')return r.abort();const file=root+'/ui'+url.pathname;if(!fs.existsSync(file))return r.respond({status:404,body:''});r.respond({status:200,contentType:file.endsWith('.js')?'text/javascript':file.endsWith('.css')?'text/css':'text/html',body:fs.readFileSync(file)});});
await page.evaluateOnNewDocument(()=>{
 let state={terms_accepted:true,allowed:false,sharing:false,notice_ready:true,terms_html:'<h1>Terms</h1>',notice_html:'<h1>Privacy</h1>'};
 window.calls=[];window.__TAURI__={core:{invoke:async(name,args)=>{window.calls.push({name,args});if(name==='privacy_status')return state;if(name==='privacy_save'){state={...state,allowed:args.shareDiagnostics,sharing:args.shareDiagnostics};return state;}if(name==='skills_list')return {skills:[{id:'agent-gibbon',name:'메모 관리',description:'메모 사용',enabled:false}],feedback:null};if(name==='manager_notes')return [{id:'sidebar-memo',title:'검증 메모',html:'<h1>Sidebar memo</h1>',capture:null}];}}};
});
for(const [width,height] of [[944,480],[600,360]]){
 await page.setViewport({width,height});await page.goto('http://gibbon.test/manager.html');await page.waitForFunction(()=>!document.getElementById('manager-navigation').hidden);
 await page.click('#settings-nav');
 await page.waitForFunction(()=>document.getElementById('privacy-setting-share').disabled===false);
 assert.equal(await page.$eval('#settings-privacy',e=>e.hidden),false);
 await page.click('#privacy-setting-share');
 const saves=()=>page.evaluate(()=>window.calls.filter(c=>c.name==='privacy_save').length);
 assert.equal(await saves(),0);
 await page.focus('#settings-skills-nav');await page.keyboard.press('Space');
 assert.equal(await page.$eval('#settings-skills-nav',e=>e.getAttribute('aria-current')),'page');
 assert.equal(await page.$eval('#settings-skills',e=>e.hidden),false);
 await page.focus('#settings-privacy-nav');await page.keyboard.press('Enter');
 assert.equal(await page.$eval('#privacy-setting-share',e=>e.checked),true);
 await page.$eval('#privacy-save',e=>e.scrollIntoView());await page.click('#privacy-save');
 await page.waitForFunction(()=>window.calls.some(c=>c.name==='privacy_save'&&c.args.shareDiagnostics));
 await page.$eval('#privacy-setting-share',e=>e.scrollIntoView());await page.click('#privacy-setting-share');
 await page.waitForFunction(()=>window.calls.some(c=>c.name==='privacy_save'&&!c.args.shareDiagnostics));
 await page.$eval('.settings-content',e=>e.scrollTop=0);
 assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
 assert(await page.$eval('.settings-content',e=>e.scrollWidth<=e.clientWidth));
 fs.mkdirSync('/tmp/gibbon-sidebar',{recursive:true});await page.screenshot({path:`/tmp/gibbon-sidebar/privacy-${width}.png`});
 await page.click('#settings-skills-nav');await page.screenshot({path:`/tmp/gibbon-sidebar/skills-${width}.png`});
 await page.focus('#settings-privacy-nav');await page.keyboard.press('Tab');assert.equal(await page.evaluate(()=>document.activeElement.id),'settings-skills-nav');
}
assert.deepEqual(errors,[]);console.log('PASS: two viewport layouts, sidebar click/Enter/Space/Tab, privacy draft, save and immediate withdrawal; no JavaScript errors.');
}finally{await browser.close();}
