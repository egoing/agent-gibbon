import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {readFileSync} from 'node:fs';
const source=readFileSync(new URL('../ui/manager-theme.js',import.meta.url),'utf8');
function setup(saved=null, {dark=false, failRead=false, failWrite=false, nativeInvoke, savedMemoDesign=null}={}) {
  let value=saved, memoValue=savedMemoDesign;
  const media={matches:dark,addEventListener(_,fn){this.change=fn;}};
  const radios=['white','gray','black','system'].map(value=>({value,checked:false,addEventListener(_,fn){this.change=fn;}}));
  const memoRadios=['white','gray','black','color','system'].map(value=>({value,checked:false,addEventListener(_,fn){this.click=fn;}}));
  const ids=Object.fromEntries(['theme-mode','theme-result','theme-native-status'].map(id=>[id,{dataset:{},setAttribute(k,v){this[k]=v;}}]));
  const doc={readyState:'loading',documentElement:{dataset:{}},getElementById:id=>ids[id],querySelectorAll:selector=>selector.includes('memo-design')?memoRadios:radios,addEventListener(_,fn){this.ready=fn;}};
  const storage={getItem(k){if(failRead)throw Error();return k==='agent-gibbon.memo-design'?memoValue:value;},setItem(k,v){if(failWrite)throw Error();if(k==='agent-gibbon.memo-design')memoValue=v;else value=v;},removeItem(){if(failWrite)throw Error();value=null;}};
  const listeners={};
  vm.runInNewContext(source,{document:doc,window:{__TAURI__:nativeInvoke?{core:{invoke:nativeInvoke}}:undefined,matchMedia:()=>media,localStorage:storage,addEventListener:(name,fn)=>listeners[name]=fn}});
  const first=doc.documentElement.dataset.managerTheme;
  doc.readyState='interactive';doc.ready();
  return {doc,ids,media,first,radios,memoRadios,listeners,storage,value:()=>value,memoValue:()=>memoValue,chooseDesign(design){const r=memoRadios.find(r=>r.value===design);r.checked=true;r.click();},choose(theme){const r=radios.find(r=>r.value===theme);r.checked=true;r.change();},system(dark){media.matches=dark;media.change();},theme:()=>doc.documentElement.dataset.managerTheme};
}
test('system light maps to gray, dark maps to black before paint and remains unsaved',()=>{
  const app=setup();assert.equal(app.first,'gray');app.system(true);assert.equal(app.theme(),'black');assert.equal(app.value(),null);app.system(false);assert.equal(app.theme(),'gray');
  assert.equal(setup(null,{dark:true}).first,'black');
});
test('all explicit themes survive reload and ignore subsequent system changes; reset resumes tracking',()=>{
  for(const theme of ['white','gray','black']) {
    const app=setup();app.choose(theme);assert.equal(app.value(),theme);app.system(true);assert.equal(app.theme(),theme);
    assert.equal(setup(app.value(),{dark:true}).first,theme);
    app.choose('system');assert.equal(app.value(),null);assert.equal(app.theme(),'black');app.system(false);assert.equal(app.theme(),'gray');
  }
});
test('corrupt or inaccessible storage follows system; write failures retain previous selection',()=>{
  assert.equal(setup('invalid',{dark:true}).theme(),'black');assert.equal(setup(null,{failRead:true}).theme(),'gray');
  const app=setup('gray',{failWrite:true});app.choose('black');assert.equal(app.theme(),'gray');assert.equal(app.value(),'gray');assert.equal(app.ids['theme-result'].dataset.error,'true');assert.match(app.ids['theme-result'].textContent,/저장하지 못/);
  const locked=setup('white',{failWrite:true});locked.choose('system');assert.equal(locked.theme(),'white');
});
test('other manager windows update or reset preference; unrelated storage is ignored',()=>{
  const app=setup();const event=newValue=>app.listeners.storage({key:'agent-gibbon.manager-theme',newValue,storageArea:app.storage});
  event('white');app.system(true);assert.equal(app.theme(),'white');event(null);assert.equal(app.theme(),'black');
  app.listeners.storage({key:'other',newValue:'gray',storageArea:app.storage});assert.equal(app.theme(),'black');
});

const tick=()=>new Promise(setImmediate);
test('native color changes are serialized and carry only a fixed theme, without delaying the UI',async()=>{
  const calls=[];let release;
  const app=setup(null,{nativeInvoke:async(name,args)=>{calls.push({name,args});if(calls.length===1)await new Promise(resolve=>release=resolve);}});
  await tick();app.choose('black');app.choose('white');assert.equal(app.theme(),'white');assert.equal(calls.length,1);
  release();await tick();
  assert.deepEqual(calls.map(c=>c.name),Array(3).fill('manager_set_theme'));
  assert.deepEqual(calls.map(c=>c.args.theme),['gray','black','white']);
  assert(calls.every(c=>Object.keys(c.args).join()==='theme'));
});
test('native failure reports a separate status and retries without undoing the saved preference',async()=>{
  let fail=true;
  const app=setup('black',{nativeInvoke:async()=>{if(fail)throw Error('native unavailable');}});
  await tick();assert.equal(app.ids['theme-native-status'].hidden,false);assert.equal(app.value(),'black');
  fail=false;app.choose('black');await tick();assert.equal(app.ids['theme-native-status'].hidden,true);assert.equal(app.theme(),'black');
});

test('five memo designs persist independently of manager selection and default to gray',()=>{
 const app=setup('black');assert.equal(app.memoRadios.find(r=>r.checked).value,'gray');
 for(const design of ['white','gray','black','system','color']){
  app.chooseDesign(design);assert.equal(app.memoValue(),design);assert.equal(app.value(),'black');
  const reload=setup('black',{savedMemoDesign:design});assert.equal(reload.memoRadios.find(r=>r.checked).value,design);
  app.choose('white');assert.equal(app.memoRadios.find(r=>r.checked).value,design);app.choose('black');
 }
 const failed=setup('gray',{failWrite:true});failed.chooseDesign('color');assert.equal(failed.memoRadios.find(r=>r.checked).value,'gray');assert.equal(failed.ids['theme-result'].dataset.error,'true');
});
test('storage changes update memo design without changing manager selection; invalid values reset only memo design',()=>{
 const app=setup('white');app.listeners.storage({key:'agent-gibbon.memo-design',newValue:'black',storageArea:app.storage});
 assert.equal(app.theme(),'white');assert.equal(app.memoRadios.find(r=>r.checked).value,'black');
 app.listeners.storage({key:'agent-gibbon.memo-design',newValue:'invalid',storageArea:app.storage});assert.equal(app.theme(),'white');assert.equal(app.memoRadios.find(r=>r.checked).value,'gray');
});

test('manager system is a radio selection and saved colors remain explicit choices',()=>{
 const app=setup(null,{dark:true});assert.equal(app.radios.find(r=>r.checked).value,'system');assert.equal(app.theme(),'black');
 app.choose('gray');app.system(true);assert.equal(app.radios.find(r=>r.checked).value,'gray');assert.equal(app.theme(),'gray');
 app.choose('system');assert.equal(app.value(),null);assert.equal(app.radios.find(r=>r.checked).value,'system');assert.equal(app.theme(),'black');
});
