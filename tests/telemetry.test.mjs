import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { readFileSync } from 'node:fs';
const source=readFileSync(new URL('../ui/telemetry.js',import.meta.url),'utf8');
const tick=()=>new Promise(r=>setImmediate(r));
function setup(enabled=true){
 const listeners={},native={},calls=[],timers=[],observers=[];let time=0,fail=false,pending=false;
 class Observer{static supportedEntryTypes=['longtask','long-animation-frame'];constructor(fn){this.fn=fn;observers.push(this);}observe(){}}
 const window={document:{visibilityState:'visible'},performance:{now:()=>time},PerformanceObserver:Observer,
  requestAnimationFrame:fn=>fn(),setInterval:(fn,ms)=>{timers.push({fn,ms});},addEventListener:(kind,fn)=>{listeners[kind]=fn;},
  __TAURI__:{event:{listen:async(kind,fn)=>{native[kind]=fn;return()=>{};}},core:{invoke:(command,args)=>{calls.push({command,args});if(command==='telemetry_status')return Promise.resolve({enabled,detailed:enabled});if(command==='telemetry_heartbeat'&&pending)return new Promise(()=>{});if(fail)return Promise.reject(Error('private error'));return Promise.resolve('value');}}}};
 Object.freeze(window.__TAURI__.core);
 vm.runInNewContext(source,{window,Map,Number});
 return{window,calls,timers,listeners,native,observers,setTime:n=>{time=n;},fail:()=>{fail=true;},pending:()=>{pending=true;}};
}
test('fixed error kinds exclude message/reason; disabled diagnostics install no timers',async()=>{
 const s=setup(false);await tick();s.listeners.error({message:'PRIVATE_NOTE'});s.listeners.unhandledrejection({reason:'PRIVATE_PATH'});await tick();
 assert.deepEqual(s.calls.map(c=>c.command),['telemetry_status']);assert(!JSON.stringify(s.calls).includes('PRIVATE'));assert.equal(s.timers.length,0);
 const allowed=setup();await tick();allowed.calls.length=0;allowed.listeners.error({message:'PRIVATE_NOTE'});allowed.listeners.unhandledrejection({reason:'PRIVATE_PATH'});await tick();assert.equal(allowed.calls.filter(c=>c.command==='report_frontend_error').length,2);assert(!JSON.stringify(allowed.calls).includes('PRIVATE'));
});
test('invoke instrumentation preserves values/rejections but never serializes arguments or errors',async()=>{
 const s=setup();await tick();const invoke=s.window.gibbonInvoke;s.calls.length=0;
 s.setTime(10);assert.equal(await invoke('save_note',{markdown:'PRIVATE_NOTE',path:'PRIVATE_PATH'}),'value');s.fail();await assert.rejects(invoke('save_note',{markdown:'PRIVATE_NOTE'}));
 const beat=s.timers.find(t=>t.ms===2000);beat.fn();await tick();const report=s.calls.find(c=>c.command==='telemetry_heartbeat');assert(report);assert(!JSON.stringify(report).includes('PRIVATE'));assert.equal(report.args.measurements[0].count,2);
});
test('lag observers send only durations and skip hidden/sleep intervals; heartbeat has one in flight',async()=>{
 const s=setup();await tick();s.calls.length=0;
 s.observers[0].fn({getEntries:()=>[{duration:123,name:'PRIVATE_URL',attribution:[{name:'PRIVATE_SCRIPT'}]}]});
 const lag=s.timers.find(t=>t.ms===1000),beat=s.timers.find(t=>t.ms===2000);s.setTime(1500);lag.fn();s.window.document.visibilityState='hidden';s.setTime(3000);lag.fn();s.setTime(100000);lag.fn();
 s.pending();beat.fn();beat.fn();const reports=s.calls.filter(c=>c.command==='telemetry_heartbeat');assert.equal(reports.length,1);assert(!JSON.stringify(reports).includes('PRIVATE'));assert.equal(reports[0].args.measurements.length,2);
});
test('captured invoke works with frozen Tauri API and source metadata is allowlisted',async()=>{
 const s=setup();const captured=s.window.gibbonInvoke;await tick();
 await captured('snapshot',{note:'PRIVATE'});s.listeners.error({filename:'/Users/PRIVATE/app.js',lineno:12,colno:3,message:'PRIVATE'});await tick();
 const e=s.calls.find(c=>c.command==='report_frontend_error');assert.equal(e.args.source,'app.js');assert.equal(e.args.line,12);assert(!JSON.stringify(e).includes('PRIVATE'));
});
test('explicit terminal stages accept only fixed names and finite visible durations',async()=>{
 const off=setup(false);await tick();assert.equal(off.window.gibbonMeasure,undefined);
 const s=setup();await tick();s.calls.length=0;
 s.window.gibbonMeasure('terminal.ui.ready',40);
 s.window.gibbonMeasure('PRIVATE_NOTE_PATH',12);
 s.window.gibbonMeasure('terminal.output.parse',Infinity);
 s.window.document.visibilityState='hidden';s.window.gibbonMeasure('terminal.first_output',10000);
 s.timers.find(t=>t.ms===2000).fn();await tick();
 const report=s.calls.find(c=>c.command==='telemetry_heartbeat');
 assert.equal(report.args.measurements.length,1);assert.equal(report.args.measurements[0].op,'terminal.ui.ready');
 assert(!JSON.stringify(report).includes('PRIVATE'));
});

test('withdrawal clears pending samples and old callbacks cannot replay after re-opt-in',async()=>{
 const s=setup();await tick();const old=s.window.gibbonMeasure;old('terminal.ui.ready',111);
 s.native['privacy-changed']({payload:{detailed:false}});assert.equal(s.window.gibbonMeasure,undefined);
 s.calls.length=0;for(const t of s.timers)t.fn();await tick();assert.equal(s.calls.some(c=>c.command==='telemetry_heartbeat'),false);
 s.native['privacy-changed']({payload:{detailed:true}});await tick();s.calls.length=0;
 old('terminal.ui.ready',111);s.window.gibbonMeasure('terminal.ui.ready',222);s.timers.at(-1).fn();await tick();
 const report=s.calls.find(c=>c.command==='telemetry_heartbeat');assert.equal(report.args.measurements.length,1);assert.equal(report.args.measurements[0].duration_ms,222);
});

test('snapshot calls keep values and failures but never create summary measurements; heartbeat survives',async()=>{
 const s=setup();await tick();s.calls.length=0;
 s.setTime(10);assert.equal(await s.window.gibbonInvoke('snapshot',{private:'PRIVATE_NOTE'}),'value');
 s.fail();await assert.rejects(s.window.gibbonInvoke('snapshot',{private:'PRIVATE_NOTE'}));
 s.timers.find(t=>t.ms===2000).fn();await tick();
 const reports=s.calls.filter(c=>c.command==='telemetry_heartbeat');
 assert.equal(reports.length,1);
 assert.equal(reports[0].args.measurements.length,0);
 assert(!JSON.stringify(reports).includes('PRIVATE'));
});
