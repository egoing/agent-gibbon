// Real WebView-independent checks for the fixed presentation script. Native
// WebKit scroller policy is checked separately in the issue177 live record.
import fs from 'node:fs';
import assert from 'node:assert/strict';
const {chromium} = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
const script = fs.readFileSync(new URL('../src-tauri/src/web_presentation.js',import.meta.url),'utf8');
const browser = await chromium.launch({headless:true});
try {
  const page=await browser.newPage();
  await page.addInitScript(script);
  await page.route('https://presentation.test/**',route=>route.fulfill({contentType:'text/html',body:`<!doctype html>${route.request().url().endsWith('/authored')?'<meta name="viewport" content="width=720">':''}<div style="height:2000px">fixture</div>`}));
  for(const [path,expected] of [['authored','width=720'],['missing','width=device-width, initial-scale=1']]) {
    await page.goto(`https://presentation.test/${path}`);
    assert.equal(await page.locator('meta[name="viewport"]').count(),1);
    assert.equal(await page.locator('meta[name="viewport"]').getAttribute('content'),expected);
    assert.equal(await page.evaluate(()=>getComputedStyle(document.documentElement,'::-webkit-scrollbar').width),'3px');
    assert.equal(await page.evaluate(()=>typeof window.__TAURI__),'undefined');
    await page.emulateMedia({forcedColors:'active'});
    assert.equal(await page.evaluate(()=>getComputedStyle(document.documentElement,'::-webkit-scrollbar').width),'auto');
    await page.emulateMedia({forcedColors:'none'});
  }
  console.log('PASS authored/missing viewport, 3px style, forced colors, no bridge; fresh navigation');
} finally {await browser.close();}
