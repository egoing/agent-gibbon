import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {readFileSync} from 'node:fs';
const flush=async()=>{for(let i=0;i<50;i++)await Promise.resolve();};
function ui({existing=false,failed=false,delayed=false,discoveryFailure=false}={}){
 const elements={},calls=[],timers=[];let resolveOpen,closed=0;
 const el=id=>elements[id]??=({id,value:'',textContent:'',children:[],attrs:{},disabled:false,setAttribute(k,v){this.attrs[k]=v;},replaceChildren(){this.children=[];},append(...v){this.children.push(...v);},addEventListener(name,fn){this[name]=fn;}});
 const invoke=async(name,args)=>{
  calls.push([name,args]);if(name==='external_welcome')return{region:[-1200,40,640,480]};
  if(name==='external_complete')return{};
  const r=args.request;
  if(r.operation==='list'){if(discoveryFailure)throw Error('discovery failed');return{apps:[{id:'a',label:'Editor'}],limit:'classic .exe'};}
  if(r.operation==='windows')return{windows:[{window:'42',title:'Existing'}]};
  if(r.operation==='sessions')return{sessions:[],launches:{}};
  if(r.operation==='attach'&&failed)throw Error('minimum size; rolled back');
  if(r.operation==='open'&&delayed)return new Promise(resolve=>{resolveOpen=resolve;});
  if(r.operation==='open'&&existing)return{existing_windows:[{window:'43',title:'Explicit target'}]};
  return{state:'attached'};
 };
 const context={setTimeout:fn=>timers.push(fn),document:{getElementById:el,createElement:()=>el('created'+Math.random())},window:{__TAURI__:{core:{invoke},window:{getCurrentWindow:()=>({close:async()=>closed++})}}}};
 vm.runInNewContext(readFileSync(new URL('../ui/external-apps.js',import.meta.url),'utf8'),context);
 return{elements,calls,timers,get closed(){return closed;},resolveOpen:value=>resolveOpen(value)};
}
test('Welcome shows installed apps and physical region without moving any window',async()=>{
 const u=ui();await flush();assert.match(u.elements.region.textContent,/-1200, 40.*640×480/);assert.equal(u.elements.apps.children[0].textContent,'Editor');
 assert.equal(u.calls.some(([,a])=>['open','attach'].includes(a?.request?.operation)),false);
 assert.equal(u.elements['run-task'].disabled,true);u.elements.apps.children[0].onclick();assert.equal(u.elements['run-task'].disabled,false);
 await u.elements['run-task'].onclick();assert.equal(u.calls.filter(([,a])=>a?.request?.operation==='open').length,1);assert.equal(u.closed,1);
});
test('existing process requires explicit window selection before attach',async()=>{
 const u=ui({existing:true});await flush();u.elements.apps.children[0].onclick();await u.elements['run-task'].onclick();
 assert.equal(u.closed,0);assert.equal(u.calls.some(([,a])=>a?.request?.operation==='attach'),false);assert.equal(u.elements['run-task'].disabled,true);
 u.elements.windows.children[0].onclick();await u.elements['run-task'].onclick();const call=u.calls.find(([,a])=>a?.request?.operation==='attach');assert.equal(call[1].request.window,'43');assert.equal(u.closed,1);
});
test('placement failure retains chooser and error, and permits explicit retry',async()=>{
 const u=ui({failed:true});await flush();u.elements.windows.children[0].onclick();await u.elements['run-task'].onclick();assert.equal(u.closed,0);assert.match(u.elements.status.textContent,/minimum size/);assert.equal(u.elements['run-task'].disabled,false);
});
test('pending launch prevents duplicate starts and cancel targets only that operation',async()=>{
 const u=ui({delayed:true});await flush();u.elements.apps.children[0].onclick();const pending=u.elements['run-task'].onclick();await flush();await u.elements['run-task'].onclick();assert.equal(u.calls.filter(([,a])=>a?.request?.operation==='open').length,1);
 u.resolveOpen({launch:'launch-own',state:'pending'});await pending;await u.elements.cancel.onclick();const cancel=u.calls.find(([,a])=>a?.request?.operation==='cancel');assert.equal(cancel[1].request.session,'launch-own');assert.equal(u.closed,1);
});
test('app discovery failure preserves existing-window selection and search',async()=>{
 const u=ui({discoveryFailure:true});await flush();assert.match(u.elements.status.textContent,/discovery failed/);assert.equal(u.elements.windows.children[0].textContent,'Existing');u.elements.search.value='missing';u.elements.search.input();assert.match(u.elements.windows.children[0].textContent,/항목이 없습니다/);
});
