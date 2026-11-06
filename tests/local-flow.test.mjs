import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {readFileSync} from 'node:fs';
const source=readFileSync(new URL('../ui/local-flow.js',import.meta.url),'utf8');
const tick=()=>new Promise(resolve=>setImmediate(resolve));
async function setup(enabled=true) {
  const calls=[], frames=[];let id=0,now=0;
  const window={document:{visibilityState:'visible'},performance:{now:()=>now},requestAnimationFrame:fn=>frames.push(fn),__TAURI__:{core:{invoke:async(command,args)=>{calls.push({command,args});if(command==='local_flow_status')return enabled;if(command==='local_flow_begin')return ++id;return 'value';}}}};
  vm.runInNewContext(source,{window,Set,Number});await tick();
  return {window,calls,frames,setTime:n=>now=n};
}
test('off opt-in neither starts a flow nor augments application arguments',async()=>{
 const s=await setup(false);s.calls.length=0;
 assert.equal(await s.window.gibbonLocalFlow('note.save',flow=>flow.invoke('save_note',{markdown:'PRIVATE'})),'value');
 assert.equal(s.calls.length,1);assert.equal(s.calls[0].args.localFlow,undefined);
});
test('overlapping flows carry distinct IDs; stages never contain application content',async()=>{
 const s=await setup();s.calls.length=0;let release;
 const first=s.window.gibbonLocalFlow('note.save',async flow=>{await new Promise(r=>release=r);await flow.invoke('save_note',{markdown:'PRIVATE'});flow.stage('PRIVATE');flow.stage('dom');});
 await tick();await s.window.gibbonLocalFlow('terminal.start',flow=>flow.invoke('terminal_start',{choice:'PRIVATE'}));release();await first;await tick();
 assert.equal(s.calls.find(c=>c.command==='save_note').args.localFlow,1);
 assert.equal(s.calls.find(c=>c.command==='terminal_start').args.localFlow,2);
 const records=s.calls.filter(c=>c.command.startsWith('local_flow'));
 assert(!JSON.stringify(records).includes('PRIVATE'));
 assert.deepEqual(records.filter(c=>c.args?.stage==='complete').map(c=>c.args.flow),[2,1]);
});
test('refresh is sampled at most once per two seconds and hidden work is uninstrumented',async()=>{
 const s=await setup();s.calls.length=0;const work=f=>f.invoke('snapshot');
 await s.window.gibbonLocalFlow('note.refresh',work);await s.window.gibbonLocalFlow('note.refresh',work);
 s.setTime(2000);await s.window.gibbonLocalFlow('note.refresh',work);
 s.window.document.visibilityState='hidden';await s.window.gibbonLocalFlow('note.save',work);
 assert.equal(s.calls.filter(c=>c.command==='local_flow_begin').length,2);
 assert.equal(s.calls.filter(c=>c.command==='snapshot').length,4);
});
test('frames do not delay application result; completion follows second callback; errors close flow',async()=>{
 const s=await setup();s.calls.length=0;
 assert.equal(await s.window.gibbonLocalFlow('note.refresh',flow=>{flow.frame();return 42;}),42);
 assert.equal(s.calls.filter(c=>c.args?.stage==='complete').length,0);
 await tick();s.frames.shift()();s.frames.shift()();await tick();
 assert.deepEqual(s.calls.filter(c=>c.command==='local_flow_stage').map(c=>c.args.stage),['frame_ready','complete']);
 const error=Error('PRIVATE');await assert.rejects(s.window.gibbonLocalFlow('note.save',()=>{throw error;}),e=>e===error);await tick();
 assert.equal(s.calls.at(-1).args.stage,'failed');assert(!JSON.stringify(s.calls).includes('PRIVATE'));
});

test('fit observation preserves application completion while frame waits for fit',async()=>{
 const s=await setup();let release;
 const fit=new Promise(resolve=>release=resolve);
 assert.equal(await s.window.gibbonLocalFlow('note.save',flow=>{flow.follow(fit);flow.frame();return 'saved';}),'saved');
 assert.equal(s.frames.length,0);release();await tick();assert.equal(s.frames.length,1);
 s.frames.shift()();s.frames.shift()();await tick();assert.equal(s.calls.at(-1).args.stage,'complete');
});
