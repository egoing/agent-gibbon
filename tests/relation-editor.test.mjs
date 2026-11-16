import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {readFileSync} from 'node:fs';
const flush=()=>new Promise(setImmediate);
function fixture(script,invoke){
 const nodes=new Map(),events={},frames=[],calls=[],native={};
 const node=id=>{
  if(!nodes.has(id))nodes.set(id,{id,attributes:{},style:{},listeners:{},value:id==='arrow'?'end':'',hidden:false,
   addEventListener(k,fn){this.listeners[k]=fn;},setAttribute(k,v){this.attributes[k]=v;},
   append(child){nodes.set(child.id,child);},focus(){this.focused=true;}});
  return nodes.get(id);
 };
 const document={getElementById:node,querySelector:()=>node('svg'),createElement:()=>({...node('new'),listeners:{},attributes:{}}),addEventListener(k,fn){events[k]=fn;}};
 const context={document,requestAnimationFrame(fn){frames.push(fn);return frames.length;},window:{__TAURI__:{core:{invoke(name,args){calls.push([name,args]);return invoke(name,args);}},event:{async listen(name,fn){native[name]=fn;}}}}};
 if(script==='relation-pick.js')vm.runInNewContext(readFileSync(new URL('../ui/relation-curve.js',import.meta.url),'utf8'),context);
 vm.runInNewContext(readFileSync(new URL(`../ui/${script}`,import.meta.url),'utf8'),context);
 return{node,events,frames,calls,native};
}
test('all memo toolbars install the accessible plug button and source Escape cancels its generation',async()=>{
 for(const file of ['index.html','terminal.html','web.html']){
  const html=readFileSync(new URL(`../ui/${file}`,import.meta.url),'utf8');
  assert.ok(html.includes('src="relation-toolbar.js"'));
  if(file==='index.html')assert.match(html,/<script defer src="relation-toolbar\.js"/);
  assert.ok(html.indexOf('src="relation-toolbar.js"')<html.indexOf('src="toolbar.js"'));
 }
 const f=fixture('relation-toolbar.js',async name=>name==='relation_status'?true:7);await flush();
 const b=f.node('connect');assert.equal(b.hidden,false);assert.equal(b.attributes['aria-label'],'메모 연결');
 assert.match(b.innerHTML,/M7 12l5 5/);
 await b.listeners.click();
 f.events.keydown({key:'Escape',preventDefault(){},stopImmediatePropagation(){}});await flush();
 assert.deepEqual(JSON.parse(JSON.stringify(f.calls.at(-1))),['relation_cancel',{generation:7}]);
 const disabled=fixture('relation-toolbar.js',async()=>false);await flush();assert.equal(disabled.node('connect').hidden,true);
});
test('picker coalesces pointer motion, snaps hover and confirms click coordinates rather than a stale hover',async()=>{
 let selected=false;
 const f=fixture('relation-pick.js',async(name,args)=>name==='relation_pick'&&args.choose?selected:true);await flush();
 f.events.pointermove({clientX:10,clientY:20});f.events.pointermove({clientX:30,clientY:40});
 await f.frames.shift()();
 assert.deepEqual(JSON.parse(JSON.stringify(f.calls.at(-1))),['relation_pick',{x:30,y:40,choose:false}]);
 f.native['relation-preview']({payload:{width:800,height:600,scale:2,points:[[1,2],[51,2],[51,8],[101,8]],target:'target',highlight:{x:90,y:80,width:100,height:120}}});
 assert.equal(f.node('line').attributes.d,'M1,2 C37,2 37,8 73,8');assert.equal(f.node('target').style.display,'block');
 const click={clientX:100,clientY:200,target:{closest:()=>false}};
 await f.events.click(click);assert.match(f.node('hint').textContent,/다른 메모/);
 selected=true;await f.events.click({...click,clientX:110});
 assert.deepEqual(JSON.parse(JSON.stringify(f.calls.at(-1))),['relation_pick',{x:110,y:200,choose:true}]);
 f.events.keydown({key:'Escape',preventDefault(){}});await flush();assert.equal(f.calls.at(-1)[0],'relation_cancel');
 assert.ok(!f.calls.some(([name])=>name==='relation_save'));
});
test('settings allow an empty label, keep direction and generation, and recover from save failure without duplicate submissions',async()=>{
 let rejectSave;
 const f=fixture('relation-settings.js',async name=>{
  if(name==='relation_editor_state')return{from:'image-1',to:'web-2',generation:22};
  if(name==='relation_save')return new Promise((_r,reject)=>{rejectSave=reject;});
 });await flush();assert.equal(f.node('label').focused,true);
 f.node('arrow').value='both';
 const first=f.node('settings').listeners.submit({preventDefault(){}});
 await f.node('settings').listeners.submit({preventDefault(){}});
 assert.equal(f.calls.filter(([name])=>name==='relation_save').length,1);
 assert.deepEqual(JSON.parse(JSON.stringify(f.calls.at(-1))),['relation_save',{label:'',arrow:'both',generation:22}]);
 f.events.keydown({key:'Escape',preventDefault(){}});assert.equal(f.calls.filter(([name])=>name==='relation_cancel').length,0);
 rejectSave(Error('대상 메모가 숨겨졌습니다'));await first;
 assert.equal(f.node('save').disabled,false);assert.match(f.node('error').textContent,/숨겨졌습니다/);
 f.events.keydown({key:'Escape',preventDefault(){}});await flush();
 assert.deepEqual(JSON.parse(JSON.stringify(f.calls.at(-1))),['relation_cancel',{generation:22}]);
});
