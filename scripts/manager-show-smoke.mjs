// Real Manager assets in Chromium; native calls and prior terms acceptance are
// mocked here. This never changes a real user's consent or native memo state.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
const { chromium } = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
const root = fileURLToPath(new URL('..', import.meta.url));
const output = process.env.GIBBON_SMOKE_OUTPUT || '/tmp/gibbon-issue68-label-browser';
fs.mkdirSync(output, { recursive: true });
const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage(), errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.route('http://gibbon.test/**', route => {
    const name = new URL(route.request().url()).pathname;
    const body = name === '/privacy.js'
      ? `window.gibbonPrivacy={accepted:()=>true,ready:Promise.resolve(),refresh:async()=>{}};
         document.addEventListener('DOMContentLoaded',()=>{
           document.getElementById('privacy-welcome').hidden=true;
           document.getElementById('manager-navigation').hidden=false;
         });`
      : fs.readFileSync(path.join(root, 'ui', name));
    return route.fulfill({ body, contentType: name.endsWith('.css') ? 'text/css' : name.endsWith('.js') ? 'text/javascript' : 'text/html' });
  });
  await page.addInitScript(() => {
    window.calls = []; window.failShow = false;
    window.rows = [
      {id:'text',title:'텍스트 검증',html:'<h1>텍스트 검증</h1><p>원본 본문</p>'},
      {id:'plain',title:'제목 없는 본문',html:'<p>제목 없는 본문</p>'},
      {id:'empty',title:'빈 메모',html:''},
      {id:'image',title:'이미지 검증',capture:{}},
      {id:'terminal',title:'터미널 검증',kind:'terminal'},
      {id:'agent',title:'에이전트 검증',kind:'agent'}
    ];
    window.__TAURI__ = { core: { invoke: async (name, args) => {
      window.calls.push({name,args});
      if (name === 'telemetry_status') return {enabled:false,detailed:false};
      if (name === 'manager_notes') return structuredClone(window.rows);
      if (name === 'manager_set_visibility' && !window.failShow) {
        window.rows.find(row=>row.id===args.id).visible=args.visible; return args.visible;
      }
      if (name === 'manager_set_visibility' && window.failShow) throw Error('PRIVATE_ERROR');
      if (name === 'manager_image') return 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+a5WQAAAAASUVORK5CYII=';
    }}};
  });
  let combinations = 0;
  for (const [width,height] of [[944,480],[600,360]]) {
    await page.setViewportSize({width,height});
    await page.goto('http://gibbon.test/manager.html');
    await page.locator('#list button').first().waitFor();
    assert.equal(await page.getByRole('button',{name:'화면에 띄우기',exact:true}).count(),0);
    for (const id of ['text','plain','empty','image','terminal','agent']) {
      const before = await page.evaluate(() => window.calls.filter(c=>c.name==='manager_set_visibility').length);
      await page.locator(`#list [data-id="${id}"]`).click();
      if (id === 'image') await page.locator('#detail img').waitFor();
      const show = page.locator('.detail-tools button');
      assert.equal(await show.textContent(),'화면에 띄우기');
      assert.equal(await page.locator('.detail-visibility').count(),0);
      assert.equal(await show.count(),1);
      assert.equal(await page.evaluate(() => window.calls.filter(c=>c.name==='manager_set_visibility').length),before);
      assert.equal(await page.locator('#detail').evaluate(el => el.firstElementChild.className),'detail-tools');
      assert(await page.locator('#detail').evaluate(el => el.scrollWidth <= el.clientWidth));
      assert(await page.locator('#detail').evaluate(el => {
        const [tools,content] = el.children;
        return tools.getBoundingClientRect().bottom <= content.getBoundingClientRect().top;
      }));
      if (id === 'text') assert.equal(await page.locator('#detail h1').count(),1);
      if (id === 'plain' || id === 'empty') assert.equal(await page.locator('#detail h1').count(),0);
      await show.focus(); await page.keyboard.press('Enter');
      await page.waitForFunction(n => window.calls.filter(c=>c.name==='manager_set_visibility').length === n,before+1);
      assert.equal(await page.evaluate(() => window.calls.filter(c=>c.name==='manager_set_visibility').at(-1).args.id),id);
      await page.getByRole('button',{name:'화면에서 숨기기',exact:true}).waitFor();
      assert.equal(await page.locator('.detail-visibility').count(),0);
      await page.waitForTimeout(2100);
      assert.equal(await page.evaluate(()=>document.activeElement===document.querySelector('.detail-tools button')),true);
      await show.click();
      await page.getByRole('button',{name:'화면에 띄우기',exact:true}).waitFor();
      assert.equal(await page.evaluate(() => window.calls.filter(c=>c.name==='manager_set_visibility').at(-1).args.visible),false);
      await show.click();
      await page.getByRole('button',{name:'화면에서 숨기기',exact:true}).waitFor();
      if (['text','image','terminal'].includes(id)) await page.screenshot({path:path.join(output,`${id}-${width}.png`)});
      combinations++;
    }
    await page.locator('.detail-tools button').focus();
    await page.evaluate(() => { window.rows.find(row=>row.id==='agent').visible=false; });
    await page.getByRole('button',{name:'화면에 띄우기',exact:true}).waitFor();
    assert.equal(await page.evaluate(()=>document.activeElement===document.querySelector('.detail-tools button')),true);
    await page.evaluate(() => { window.failShow = true; });
    await page.getByRole('button',{name:'화면에 띄우기',exact:true}).click();
    const alert = page.locator('.detail-error'); await alert.waitFor({state:'visible'});
    assert.match(await alert.textContent(),/변경하지 못했습니다/);
    assert.doesNotMatch(await alert.textContent(),/PRIVATE/);
    await page.waitForTimeout(2200); assert(await alert.isVisible());
    await page.screenshot({path:path.join(output,`error-${width}.png`)});
    await page.evaluate(() => { window.failShow = false; });
    await page.getByRole('button',{name:'화면에 띄우기',exact:true}).click();
    await alert.waitFor({state:'hidden'});
    await page.locator('#settings-nav').click();
    assert.equal(await page.getByRole('button',{name:'화면에 띄우기',exact:true}).count(),0);
    await page.locator('#notes-nav').click();
    assert.equal(await page.getByRole('button',{name:'화면에서 숨기기',exact:true}).count(),1);
    await page.evaluate(() => { window.rows = []; }); await page.locator('#refresh').click();
    await page.getByText('아직 메모가 없습니다.',{exact:true}).first().waitFor();
    assert.equal(await page.getByRole('button',{name:'화면에 띄우기',exact:true}).count(),0);
  }
  assert.deepEqual(errors,[]);
  console.log(JSON.stringify({browser:browser.version(),combinations,errors,output}));
} finally { await browser.close(); }
