import { sourceLanguageWindow } from './fixtures/language.mjs';
import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { readFileSync } from 'node:fs';
const source = readFileSync(new URL('../ui/developer-updater.js', import.meta.url), 'utf8');
const tick = () => new Promise(setImmediate);
function setup(initial = { phase: 'idle', current_version: '1.0.0' }) {
  const elements = new Map();
  const make = () => ({ hidden: false, disabled: false, textContent: '', children: [], replaceChildren() { this.children = []; }, append(...children) { this.children.push(...children); } });
  for (const id of ['settings-version', 'settings-developer-nav', 'settings-page', 'settings-developer', 'developer-update-state', 'developer-update-message', 'developer-prepare', 'developer-install', 'developer-previous', 'developer-latest', 'developer-roundtrip-reason']) elements.set(id, make());
  elements.get('settings-developer-nav').hidden = true;
  let now = 0, status = initial, invokeAction = async () => status;
  const requests = [];
  const payloads = [];
  const context = vm.createContext(sourceLanguageWindow({ document: { hidden: false, getElementById: id => elements.get(id), createElement: make }, performance: { now: () => now }, window: { jajanInvoke: async (name, args) => { requests.push(name); payloads.push({name,args}); return name === 'updater_status' ? status : invokeAction(name); } }, setInterval() {} }));
  vm.runInContext(source, context);
  return { elements, requests, payloads, click: delta => { now += delta; elements.get('settings-version').onclick(); }, setStatus: value => { status = value; }, setAction: fn => { invokeAction = fn; }, refresh: () => context.window.jajanDeveloperUpdater.refresh() };
}
test('developer entry and refresh never invoke deferred terminal separation APIs', async () => {
  const app=setup(); await tick();
  for(let i=0;i<5;i++) app.click(100);
  await app.refresh();
  assert.equal(app.elements.get('settings-developer-nav').hidden,false);
  assert.equal(app.requests.some(name=>name.startsWith('terminal_')),false);
});
test('public builds cannot downgrade without a menu-compatible pair; isolated pair reverses direction after restart', async () => {
  const app = setup({phase:'idle',current_version:'0.3.12'}); await tick();
  assert.equal(app.elements.get('developer-previous').disabled,true);
  assert.equal(app.elements.get('developer-latest').disabled,true);
  assert.match(app.elements.get('developer-roundtrip-reason').textContent,/공식 호환 버전 쌍/);
  const pair={previous:'0.3.12',latest:'0.3.13'};
  app.setStatus({phase:'ready',current_version:'0.3.13',roundtrip_pair:pair}); await app.refresh();
  assert.equal(app.elements.get('developer-previous').disabled,false);
  assert.equal(app.elements.get('developer-latest').disabled,true);
  await app.elements.get('developer-previous').onclick();
  assert.equal(app.payloads.at(-1).name,'updater_test_roundtrip');
  assert.equal(app.payloads.at(-1).args.destination,'previous');
  app.setStatus({phase:'ready',current_version:'0.3.12',roundtrip_pair:pair,manual_test:true}); await app.refresh();
  assert.equal(app.elements.get('developer-previous').disabled,true);
  assert.equal(app.elements.get('developer-latest').disabled,false);
  await app.elements.get('developer-latest').onclick();
  assert.equal(app.payloads.at(-1).args.destination,'latest');
});
test('five rapid clicks reveal once; slow clicks reset; refresh preserves reveal', async () => {
  const app = setup(); await tick();
  app.click(0); app.click(100); app.click(100); app.click(601);
  for (let i = 0; i < 3; i++) app.click(100);
  assert.equal(app.elements.get('settings-developer-nav').hidden, true);
  app.click(100);
  assert.equal(app.elements.get('settings-developer-nav').hidden, false);
  await app.refresh();
  app.click(1000);
  assert.equal(app.elements.get('settings-developer-nav').hidden, false);
  assert.equal(app.elements.get('settings-version').textContent, 'v1.0.0');
});
test('prepare is explicit, rejects duplicate actions and never installs on ready refresh', async () => {
  const app = setup(); await tick();
  let resolve;
  app.setAction(() => new Promise(r => { resolve = r; }));
  const preparing = app.elements.get('developer-prepare').onclick();
  app.elements.get('developer-prepare').onclick();
  assert.equal(app.requests.filter(n => n === 'updater_test_prepare').length, 1);
  const ready = {phase:'ready', current_version:'1.0.0', version:'1.0.0', manual_test:true, signature_verified:true, cache_ready:true, downloaded:42, total:42, message:'수동 시험 대기'};
  app.setStatus(ready); resolve(ready); await preparing;
  await app.refresh();
  assert.equal(app.elements.get('developer-install').disabled, false);
  assert.match(app.elements.get('developer-update-state').children.map(n => n.textContent).join(' '), /동일 버전 재설치 테스트/);
  assert.equal(app.requests.some(n => n === 'updater_test_install'), false);
});
test('installation preserves precise protection reason, disabled builds cannot prepare', async () => {
  const app = setup({phase:'ready', current_version:'1.0.0', version:'1.0.0', manual_test:true}); await tick();
  app.setAction(async () => ({phase:'ready', manual_test:true, message:'편집을 저장한 뒤 다시 시도하세요.'}));
  await app.elements.get('developer-install').onclick();
  assert.equal(app.elements.get('developer-update-message').textContent, '편집을 저장한 뒤 다시 시도하세요.');
  app.setStatus({phase:'disabled'}); await app.refresh();
  assert.equal(app.elements.get('developer-prepare').disabled, true);
  assert.equal(app.elements.get('developer-install').disabled, true);
});
test('normal ready package cannot be installed as a test, and errors remain visible', async () => {
  const app = setup({phase:'ready', manual_test:false}); await tick();
  assert.equal(app.elements.get('developer-install').disabled, true);
  app.setStatus({phase:'error', message:'캐시 저장 실패'});
  app.setAction(async () => { throw Error('private detail'); });
  await app.elements.get('developer-prepare').onclick();
  assert.match(app.elements.get('developer-update-message').textContent, /캐시 저장 실패/);
  assert.doesNotMatch(app.elements.get('developer-update-message').textContent, /private detail/);
});
