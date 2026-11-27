import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
const flush = async () => { for (let i=0; i<8; i++) await Promise.resolve(); };
function selectionUI() {
  const events = {}, calls = [], frames = [];
  const elements = Object.fromEntries(['selection','hint','screen','drag-shade'].map(id => [id,{style:{},hidden:true}]));
  const shadeClasses = new Set();
  elements['drag-shade'].classList = {add: name => shadeClasses.add(name), remove: name => shadeClasses.delete(name)};
  const ticks = [], classes = new Set();
  const context = {requestAnimationFrame: cb=>ticks.push(cb), innerWidth:1000, innerHeight:800,
    ImageData: class { constructor(data,width,height) { Object.assign(this,{data,width,height}); } },
    window:{addEventListener:(name,fn)=>{events[name]=fn;}, __TAURI__:{ core:{ invoke:async (name,args) => {
      calls.push([name,args]);
    } } } },
    document:{documentElement:{classList:classes},getElementById:id=>elements[id],body:{setPointerCapture(){},hasPointerCapture(){return true;},releasePointerCapture(id){ calls.push(['releasePointerCapture',id]); }},addEventListener:(name,fn)=>{events[name]=fn;}},
  };
  vm.runInNewContext(readFileSync(new URL('../ui/capture.js',import.meta.url),'utf8'),context);
  const ready = flush;
  return {events,calls,elements,ready,frames,ticks,classes};
}
test('reverse drag clamps to the display and submits one selection',async()=>{
  const {events,calls,ready} = selectionUI(); await ready();
  events.pointerdown({button:0,clientX:400,clientY:300,pointerId:1});
  events.pointermove({clientX:-20,clientY:100});
  await events.pointerup(); await events.pointerup();
  const results = calls.filter(([name])=>name==='finish_capture');
  assert.equal(results.length,1);
  assert.deepEqual(JSON.parse(JSON.stringify(results[0][1])),{x:0,y:100,width:400,height:200});
});
test('click without drag creates no note; Escape cancels',async()=>{
  const {events,calls,ready} = selectionUI(); await ready();
  events.pointerdown({button:0,clientX:40,clientY:30,pointerId:1}); await events.pointerup();
  events.keydown({preventDefault(){},key:'Escape'});
  assert.equal(calls.filter(([name])=>name==='finish_capture').length,0);
  assert.equal(calls.filter(([name])=>name==='cancel_capture').length,1);
});
test('selection becomes ready without requesting any screen pixels', async()=>{
  const {calls,ready} = selectionUI(); await ready();
  assert.deepEqual(calls.map(([name])=>name),['capture_ready']);
});
test('the first pointer press after paint immediately starts selection',async()=>{
  const {events,calls,ready} = selectionUI(); await ready();
  events.pointerdown({button:0,clientX:10,clientY:20,pointerId:1});
  events.pointermove({clientX:80,clientY:90}); await events.pointerup();
  const result=calls.find(([name])=>name==='finish_capture');
  assert.deepEqual(JSON.parse(JSON.stringify(result[1])),{x:10,y:20,width:70,height:70});
});

test('the first Escape cancels without a click even while shortcut modifiers are held',async()=>{
  const {events,calls,ready} = selectionUI(); await ready();
  events.keydown({preventDefault(){},key:'Escape',ctrlKey:true,shiftKey:true});
  events.keydown({preventDefault(){},key:'Escape',repeat:true});
  assert.equal(calls.filter(([name])=>name==='cancel_capture').length,1);
});

test('Escape releases an active drag before pointerup can commit it',async()=>{
  const {events,calls,ready} = selectionUI(); await ready();
  events.pointerdown({button:0,clientX:10,clientY:20,pointerId:7});
  events.pointermove({clientX:80,clientY:90});
  events.keydown({preventDefault(){},key:'Escape'});
  await events.pointerup();
  assert.deepEqual(calls.slice(-2),[['releasePointerCapture',7],['cancel_capture',undefined]]);
});

test('drag shows the selection border with no presentation event or animation frame', async () => {
  const {events, ready, ticks, elements} = selectionUI();
  await ready();
  events.pointerdown({button:0,clientX:15,clientY:20,pointerId:1});
  events.pointermove({clientX:40,clientY:60});
  assert.equal(elements.selection.hidden, false);
  assert.equal(elements.hint.hidden, true);
  assert.equal(ticks.length, 0);
});
