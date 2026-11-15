import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { readFileSync } from 'node:fs';
function setup() {
  const nodes = new Map(), calls = [], native = {};
  let pending, fail = false;
  const node = id => {
    if (!nodes.has(id)) nodes.set(id, {
      listeners: {}, hidden: false, attributes: {}, clientWidth: 300, offsetWidth: 24, scrollWidth: 100, scrollLeft: 0,
      addEventListener(name, callback) { this.listeners[name] = callback; },
      querySelectorAll: () => [], setAttribute(k,v) { this.attributes[k]=v; },
      getAttribute(k) { return this.attributes[k]; }, removeAttribute(k) { delete this.attributes[k]; },
    });
    return nodes.get(id);
  };
  const applied=[];
  vm.runInNewContext(readFileSync(new URL('../ui/toolbar.js', import.meta.url), 'utf8'), {
    document: {getElementById: id => id === 'fold-title' ? null : node(id), body: {classList: {contains: () => false, toggle(){}}}},
    window: {gibbonApplyFold: value => applied.push(value), __TAURI__: {
      webviewWindow: {getCurrentWebviewWindow: () => ({listen(name, fn) { native[name]=fn; }})},
      core: {invoke: async name => { calls.push(name); if(fail) throw Error('native failed'); return pending ? await pending : false; }},
    }}, ResizeObserver: class {observe(){}}, MutationObserver: class {observe(){}},
  });
  const event = (patch={}) => ({button:0,buttons:1,pointerId:3,clientX:100,clientY:10,preventDefault(){},stopPropagation(){},target:{closest:()=>null},...patch});
  return {calls,applied,node,event,send: (type,patch) => node('controls').listeners[type](event(patch)),
    fail(value) {fail=value;}, wait(promise) {pending=promise;}};
}
test('stationary toolbar clicks stay local; double click toggles once and synchronizes folded content', async () => {
 const app=setup();
 for(let i=0;i<2;i++) {app.send('pointerdown');app.send('pointerup');}
 assert.deepEqual(app.calls,[]);
 await app.send('dblclick');
 assert.deepEqual(app.calls,['chrome_work_area']);
 assert.deepEqual(app.applied,[false]);
});
test('toolbar drag requires held primary pointer movement and cancels after release/cancellation', () => {
 const app=setup();
 app.send('pointerdown');app.send('pointermove',{clientX:102});assert.deepEqual(app.calls,[]);
 app.send('pointermove',{clientX:105});app.send('pointermove',{clientX:110});
 assert.deepEqual(app.calls,['chrome_drag']);
 for(const type of ['pointerup','pointercancel']) {
   app.send('pointerdown');app.send(type);app.send('pointermove',{clientX:120});
 }
 app.send('pointerdown');app.send('pointermove',{buttons:0,clientX:120});
 app.send('pointerdown',{button:2});app.send('pointermove',{clientX:120});
 assert.deepEqual(app.calls,['chrome_drag']);
});
test('buttons and editable descendants keep gestures; titles drag but never maximize', async () => {
 const app=setup();
 const control={closest: selector => selector.includes('button') ? {} : null};
 app.send('pointerdown',{target:control});app.send('pointermove',{clientX:110});
 await app.send('dblclick',{target:control});
 await app.send('dblclick',{target:{closest:selector=>selector==='#chrome-title'?{}:null}});
 assert.deepEqual(app.calls,[]);
 app.send('pointerdown',{target:{closest:selector=>selector==='#chrome-title'?{}:null}});
 app.send('pointermove',{clientX:110});assert.deepEqual(app.calls,['chrome_drag']);
});
test('a pending toggle cannot race another; native failure is reported and retry works', async () => {
 const app=setup();let resolve;
 app.wait(new Promise(done=>{resolve=done;}));
 const first=app.send('dblclick');await app.send('dblclick');
 assert.deepEqual(app.calls,['chrome_work_area']);resolve(true);await first;
 assert.deepEqual(app.applied,[true]);
 app.wait(null);app.fail(true);await app.send('dblclick');
 assert.match(app.node('status').textContent,/native failed/);
 app.fail(false);await app.send('dblclick');assert.equal(app.calls.length,3);
});
