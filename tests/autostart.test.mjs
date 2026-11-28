import { sourceLanguageWindow } from './fixtures/language.mjs';
import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { readFileSync } from 'node:fs';
const source=readFileSync(new URL('../ui/autostart.js',import.meta.url),'utf8');
const html=readFileSync(new URL('../ui/manager.html',import.meta.url),'utf8');
const tick=()=>new Promise(setImmediate);
function setup(initial={}, rejectSave=false, failRead=false) {
  const elements=new Map();
  for(const [,id] of html.matchAll(/\bid="([^"]+)"/g)) elements.set(id,{hidden:true,checked:false,disabled:true,textContent:'',events:{},addEventListener(name,fn){this.events[name]=fn;}});
  let state={selected:null,available:true,os_status:'disabled',...initial};
  const calls=[],events={};
  const window={addEventListener:(name,fn)=>events[name]=fn,__TAURI__:{core:{invoke:async(name,args)=>{
    calls.push({name,args});
    if(name==='autostart_status') { if(failRead) throw Error('status unavailable'); return {...state}; }
    if(name==='autostart_save') {
      state={...state,selected:args.enabled};
      if(rejectSave) throw Error('registration denied');
      state.os_status=args.enabled?'registered_unknown':'disabled';return {...state};
    }
  }}}};
  vm.runInNewContext(source,sourceLanguageWindow({window,document:{getElementById:id=>elements.get(id)},Boolean,Error}));
  return {elements,calls,window,events,setState:next=>state={...state,...next},setReadFailure:value=>failRead=value};
}
test('first run proposes checked startup, but reads/focus never register',async()=>{
  const s=setup();await tick();
  assert.equal(s.elements.get('autostart-welcome').checked,true);
  assert.equal(s.elements.get('autostart-setting').checked,false);
  await s.window.jajanAutostart.refresh();
  assert.equal(s.calls.some(c=>c.name==='autostart_save'),false);
  await s.window.jajanAutostart.confirmWelcome();
  assert.equal(s.calls.find(c=>c.name==='autostart_save').args.enabled,true);
  assert.match(s.elements.get('autostart-status').textContent,/허용 여부 미확인/);
  assert.doesNotMatch(s.elements.get('autostart-status').textContent,/허용했습니다/);
});
test('onboarding rejection is explicit Off and does not alter diagnostics',async()=>{
  const s=setup();await tick();const box=s.elements.get('autostart-welcome');
  box.checked=false;box.events.change();
  await s.window.jajanAutostart.confirmWelcome();
  assert.equal(s.calls.find(c=>c.name==='autostart_save').args.enabled,false);
  assert.equal(s.calls.some(c=>c.name.startsWith('privacy_')),false);
});
test('existing Off and OS-disabled On remain distinct after refresh/restart',async()=>{
  const off=setup({selected:false});await tick();
  assert.equal(off.elements.get('autostart-welcome').checked,false);
  const s=setup({selected:true,os_status:'requires_approval'});await tick();
  assert.equal(s.elements.get('autostart-setting').checked,true);
  assert.match(s.elements.get('autostart-status').textContent,/비활성화/);
  await s.window.jajanAutostart.refresh();
  assert.equal(s.calls.some(c=>c.name==='autostart_save'),false);
});
test('development/verification and unknown status never submit default On',async()=>{
  const s=setup({available:false,os_status:'unavailable'});await tick();
  assert.equal(s.elements.get('autostart-welcome').disabled,true);
  await s.window.jajanAutostart.confirmWelcome();
  await s.elements.get('autostart-setting').events.change();
  assert.equal(s.calls.some(c=>c.name==='autostart_save'),false);
  assert.equal(s.elements.get('autostart-os-settings').hidden,true);
});
test('settings changes apply immediately; OS error preserves intent and reports mismatch',async()=>{
  const s=setup({selected:true},true);await tick();const box=s.elements.get('autostart-setting');
  box.checked=false;const pending=box.events.change();
  assert.equal(s.window.jajanAutostart.updateBusy(),true);
  assert.equal(s.calls.filter(c=>c.name==='autostart_save').length,1);
  assert.equal(box.disabled,true);
  await pending;
  assert.equal(s.window.jajanAutostart.updateBusy(),false);
  assert.equal(box.checked,false);
  assert.equal(s.elements.get('autostart-error').hidden,false);
  assert.match(s.elements.get('autostart-error').textContent,/다를 수 있습니다/);
  assert.match(s.elements.get('autostart-choice').textContent,/끄기/);
});
test('onboarding OS failure is returned to the global consent flow',async()=>{
  const s=setup({},true);await tick();
  await assert.rejects(s.window.jajanAutostart.confirmWelcome(),/자동 실행 설정을 적용하지 못했습니다/);
});
test('successful checkbox change saves once and refresh never repeats it',async()=>{
  const s=setup({selected:false});await tick();const box=s.elements.get('autostart-setting');
  box.checked=true;await box.events.change();
  s.setState({os_status:'requires_approval'});await s.window.jajanAutostart.refresh();
  assert.equal(box.checked,true);assert.equal(s.window.jajanAutostart.updateBusy(),false);
  assert.equal(s.calls.filter(c=>c.name==='autostart_save').length,1);
});
test('OS settings action only appears for approval-required or unconfirmed states',async()=>{
  for(const os_status of ['disabled','enabled','requires_approval','registered_unknown','unavailable']){
    const s=setup({os_status,available:os_status!=='unavailable'});await tick();
    assert.equal(s.elements.get('autostart-os-settings').hidden,!['requires_approval','registered_unknown'].includes(os_status));
    assert.equal(s.elements.get('autostart-retry').hidden,true);
    assert.doesNotMatch(s.elements.get('autostart-status').textContent,/개발·검증 앱/);
  }
});
test('failed status read exposes only retry and retry does not register',async()=>{
  const s=setup({},false,true);await tick();
  assert.equal(s.elements.get('autostart-retry').hidden,false);
  assert.equal(s.elements.get('autostart-setting').disabled,true);
  assert.equal(s.elements.get('autostart-os-settings').hidden,true);
  s.setReadFailure(false);await s.elements.get('autostart-retry').onclick();
  assert.equal(s.elements.get('autostart-retry').hidden,true);
  assert.equal(s.elements.get('autostart-setting').disabled,false);
  assert.equal(s.calls.some(c=>c.name==='autostart_save'),false);
});
test('settings places privacy last among default destinations and keeps developer hidden',()=>{
  const nav=html.match(/<nav aria-label="설정 항목"[^>]*>([\s\S]*?)<\/nav>/)[1];
  assert.deepEqual([...nav.matchAll(/id="settings-([^"]+)-nav"/g)].map(m=>m[1]),['skills','language','appearance','updater','autostart','privacy','developer']);
  assert.match(nav, /id="settings-developer-nav"[^>]* hidden/);
  assert.doesNotMatch(html,/id="autostart-(save|refresh)"/);
});
