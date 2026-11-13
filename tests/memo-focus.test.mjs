import {test} from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {readFileSync} from 'node:fs';
const source=readFileSync(new URL('../ui/memo-focus.js',import.meta.url),'utf8');
for (const stale of [false,true]) test(`native memo focus follows events and protects initial sample (race=${stale})`,async()=>{
  let listener, resolveInitial;
  const states=[];
  const initial=new Promise(resolve=>{resolveInitial=resolve});
  const win={label:"note-proof"};
  const run=vm.runInNewContext(`(async()=>{${source}})()`,{window:{__TAURI__:{window:{getCurrentWindow:()=>win},event:{listen:async (_name,fn)=>{listener=fn}},core:{invoke:()=>initial}}},document:{body:{classList:{toggle:(name,value)=>states.push([name,value])}}}});
  await new Promise(resolve=>setImmediate(resolve));
  if(stale)listener({payload:false});
  resolveInitial(true);await run;
  assert.deepEqual(states,[['memo-focused',!stale]]);
  listener({payload:false});listener({payload:true});
  assert.deepEqual(states.slice(-2),[['memo-focused',false],['memo-focused',true]]);
});
