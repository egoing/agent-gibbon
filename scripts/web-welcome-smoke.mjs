// Actual Welcome assets in Chromium. No server or native IPC is started.
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
const {default:puppeteer}=await import(process.env.PUPPETEER_MODULE||'puppeteer-core');
const root=fileURLToPath(new URL('..',import.meta.url));
const out=process.env.WEB_OUTPUT||'/tmp/gibbon-web-welcome';fs.mkdirSync(out,{recursive:true});
const browser=await puppeteer.launch({executablePath:process.env.CHROME_BIN||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',headless:true});
const results=[];
try {
 const page=await browser.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.setRequestInterception(true);page.on('request',request=>{const url=new URL(request.url());assert.equal(url.hostname,'gibbon.test');const file=root+'ui'+url.pathname;if(!fs.existsSync(file))return request.respond({status:404,body:''});request.respond({status:200,contentType:file.endsWith('.css')?'text/css':file.endsWith('.js')?'text/javascript':'text/html',body:fs.readFileSync(file)});});
 await page.evaluateOnNewDocument(()=>{window.calls=[];window.__TAURI__={core:{invoke:async(name,args)=>{window.calls.push({name,args});throw Error('URL 검증 오류');}}};});
 for(const [width,height] of [[720,460],[320,240],[240,160]]) {
  await page.setViewport({width,height});await page.goto('http://gibbon.test/web.html');
  assert.equal(await page.evaluate(()=>window.calls.length),0);
  await page.locator('#web-url').fill('https://example.com/');await page.locator('#run').click();
  await page.waitForFunction(()=>document.getElementById('status').textContent.includes('URL 검증 오류'));
  assert.equal(await page.$eval('#web-url',e=>e.value),'https://example.com/');assert.equal(await page.$eval('#run',e=>e.disabled),false);
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
  await page.locator('#web-url').fill('https://example.org/');await page.focus('#web-url');await page.keyboard.press('Enter');
  await page.waitForFunction(()=>window.calls.length===2);
  await page.screenshot({path:`${out}/welcome-${width}.png`});results.push({width,height,calls:await page.evaluate(()=>window.calls)});
 }
 assert.deepEqual(errors,[]);fs.writeFileSync(`${out}/report.json`,JSON.stringify({results,errors},null,2));console.log('PASS: 3 viewport sizes, submit/Enter, error/input preservation, no horizontal overflow');
} finally {await browser.close();}
