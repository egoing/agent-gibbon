import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
const flush = async () => { for (let i=0; i<8; i++) await Promise.resolve(); };
function selectionUI({ malformed = false, noCanvas = false } = {}) {
  const events = {}, calls = [], frames = [];
  const elements = Object.fromEntries(['selection','hint','screen','shade'].map(id => [id,{style:{},hidden:true}]));
  const pixels = new Uint8Array([2,0,0,0,1,0,0,0,255,0,1,255,2,3,254,255]);
  let deliver;
  elements.screen.getContext = () => noCanvas ? null : { putImageData: (frame,x,y) => { frames.push({frame,x,y}); calls.push(['paint']); } };
  const context = { innerWidth:1000, innerHeight:800,
    ImageData: class { constructor(data,width,height) { Object.assign(this,{data,width,height}); } },
    window:{ __TAURI__:{ core:{ invoke:async (name,args) => {
      calls.push([name,args]);
      if (name==='capture_screen') return new Promise(resolve => { deliver=()=>resolve(malformed ? new ArrayBuffer(8) : pixels.buffer); });
    } } } },
    document:{getElementById:id=>elements[id],body:{setPointerCapture(){}},addEventListener:(name,fn)=>{events[name]=fn;}},
  };
  vm.runInNewContext(readFileSync(new URL('../ui/capture.js',import.meta.url),'utf8'),context);
  const ready = async () => { deliver(); await flush(); };
  return {events,calls,elements,ready,frames};
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
  events.keydown({key:'Escape'});
  assert.equal(calls.filter(([name])=>name==='finish_capture').length,0);
  assert.equal(calls.filter(([name])=>name==='cancel_capture').length,1);
});
test('original RGBA pixels are painted before showing, without a load or click event',async()=>{
  const {calls,ready,frames,elements} = selectionUI(); await flush();
  assert.equal(calls.some(([name])=>name==='capture_ready'),false);
  await ready();
  assert.deepEqual(calls.map(([name])=>name),['capture_screen','paint','capture_ready']);
  assert.equal(elements.screen.width,2); assert.equal(elements.screen.height,1);
  assert.deepEqual(Array.from(frames[0].frame.data),[255,0,1,255,2,3,254,255]);
  assert.equal(frames[0].x,0); assert.equal(frames[0].y,0);
});
for (const options of [{malformed:true},{noCanvas:true}]) {
  test(`unusable preview cancels without displaying a black overlay: ${JSON.stringify(options)}`,async()=>{
    const {calls,ready} = selectionUI(options); await ready();
    assert.equal(calls.some(([name])=>name==='capture_ready'),false);
    assert.equal(calls.filter(([name])=>name==='capture_failed').length,1);
  });
}
test('the first pointer press after paint immediately starts selection',async()=>{
  const {events,calls,ready} = selectionUI(); await ready();
  events.pointerdown({button:0,clientX:10,clientY:20,pointerId:1});
  events.pointermove({clientX:80,clientY:90}); await events.pointerup();
  const result=calls.find(([name])=>name==='finish_capture');
  assert.deepEqual(JSON.parse(JSON.stringify(result[1])),{x:10,y:20,width:70,height:70});
});
