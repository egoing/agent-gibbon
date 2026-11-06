import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {readFileSync} from 'node:fs';
const source=readFileSync(new URL('../ui/skills.js',import.meta.url),'utf8');
const tick=()=>new Promise(setImmediate);
function setup(){
 const make=()=>({children:[],dataset:{},hidden:false,disabled:false,attributes:{},append(...v){this.children.push(...v);},setAttribute(k,v){this.attributes[k]=v;}});
 const elements=new Map(['skills-feedback','skills-list','skills-loading'].map(id=>[id,make()]));
 let skills=[{id:'agent-gibbon',name:'메모 관리',description:'메모 사용',enabled:false},{id:'agent-gibbon-progress',name:'작업 진행 표시',description:'진행 표시',enabled:false}],pending=null,fail=false,failRead=false;
 const calls=[];const window={__TAURI__:{core:{invoke:async(name,args)=>{
  calls.push({name,args});
  if(name==='skills_list'&&failRead)throw Error('조회 실패');
  if(name==='skills_set'){
   if(pending)await pending;
   if(fail)throw Error('설치 실패');
   skills=skills.map(s=>s.id===args.id?{...s,enabled:args.enabled}:s);
  }
  return {skills:skills.map(s=>({...s})),feedback:null};
 }}}};
 vm.runInNewContext(source,{window,document:{getElementById:id=>elements.get(id),createElement:make},Map,String});
 const rows=()=>elements.get('skills-list').children;
 return {window,elements,calls,rows,toggle:i=>rows()[i].children[1].children[1],delay:()=>{let release;pending=new Promise(r=>release=r);return release;},fail:()=>{fail=true;},readFailure:value=>{failRead=value;}};
}
test('skills are lazy and independent; immediate toggle preserves accordion state',async()=>{
 const app=setup();assert.equal(app.calls.length,0);await app.window.gibbonSkills.refresh();
 assert.equal(app.rows().length,2);app.rows()[0].open=true;
 await app.toggle(0).onclick();await tick();
 assert.equal(app.calls.filter(c=>c.name==='skills_set').length,1);
 assert.equal(app.rows()[0].dataset.enabled,'true');assert.equal(app.rows()[1].dataset.enabled,'false');
 assert.equal(app.toggle(0).attributes['aria-checked'],'true');assert.equal(app.toggle(0).children[0].textContent,'On');
 assert.equal(app.rows()[0].open,true);
 await app.toggle(0).onclick();await tick();assert.equal(app.rows()[0].dataset.enabled,'false');
});
test('pending install has no optimistic On and prevents duplicate changes',async()=>{
 const app=setup();await app.window.gibbonSkills.refresh();const release=app.delay();
 app.toggle(0).onclick();app.toggle(1).onclick();await tick();
 assert.equal(app.toggle(0).attributes['aria-disabled'],'true');assert.equal(app.toggle(1).attributes['aria-disabled'],'true');
 assert.equal(app.rows()[0].dataset.enabled,'false');
 assert.equal(app.calls.filter(c=>c.name==='skills_set').length,1);
 release();await tick();assert.equal(app.rows()[0].dataset.enabled,'true');assert.equal(app.toggle(0).disabled,false);
});
test('installation failure reloads authoritative state and reports the error',async()=>{
 const app=setup();await app.window.gibbonSkills.refresh();app.fail();
 app.toggle(0).onclick();await tick();await tick();
 assert.equal(app.rows()[0].dataset.enabled,'false');assert.equal(app.toggle(0).attributes['aria-checked'],'false');
 assert.match(app.elements.get('skills-feedback').textContent,/설치 실패/);assert.equal(app.elements.get('skills-feedback').hidden,false);
 assert.equal(app.calls.at(-1).name,'skills_list');assert.equal(app.toggle(0).disabled,false);
});

test('failed initial load ends loading state and navigation can retry',async()=>{
 const app=setup();app.readFailure(true);await app.window.gibbonSkills.refresh();
 assert.equal(app.elements.get('skills-loading').hidden,true);assert.match(app.elements.get('skills-feedback').textContent,/조회 실패/);
 app.readFailure(false);await app.window.gibbonSkills.refresh();assert.equal(app.rows().length,2);assert.equal(app.elements.get('skills-feedback').hidden,true);
});
