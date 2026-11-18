import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {readFileSync} from 'node:fs';
function chrome({welcome=true, failZoom=false}={}) {
 const nodes=new Map(), calls=[];let alwaysOnTop=true;
 const node=id=>{
  if(!nodes.has(id)) nodes.set(id,{textContent:'',attributes:{},listeners:{},setAttribute(name,value){this.attributes[name]=value;},addEventListener(name,callback){this.listeners[name]=callback;}});
  return nodes.get(id);
 };
 const directions=['North','South','East','West','NorthWest','NorthEast','SouthWest','SouthEast'];
 const handles=directions.map(direction=>({...node(direction),dataset:{direction}}));
 const context={
  document:{getElementById:node,querySelector:()=>node('main'),querySelectorAll:()=>handles},
  window:{__TAURI__:{core:{invoke:async(name,args)=>{calls.push({name,...args});if(args.action==='pin')alwaysOnTop=!alwaysOnTop;if(failZoom && args.action==='zoom')throw Error('배율 실패');return {welcome,always_on_top:alwaysOnTop};}}}},
 };
 for(const script of ['content-size.js','web-chrome.js'])vm.runInNewContext(readFileSync(new URL('../ui/'+script,import.meta.url),'utf8'),context);
 return {nodes,calls,handles};
}
test('trusted web chrome pin toggles through web_chrome and reflects native state',async()=>{
 const app=chrome();
 await Promise.resolve();
 assert.equal(app.nodes.get('pin').attributes['aria-pressed'],'true');
 app.nodes.get('pin').onclick();await Promise.resolve();
 assert.deepEqual(app.calls.map(call=>call.action),['inspect','pin']);
 assert.equal(app.calls[1].name,'web_chrome');
 assert.equal(app.nodes.get('pin').attributes['aria-pressed'],'false');
});
test('trusted web chrome X dispatches close through web_chrome',async()=>{
 const app=chrome();
 app.nodes.get('close').onclick();await Promise.resolve();
 assert.equal(app.calls.at(-1).name,'web_chrome');
 assert.equal(app.calls.at(-1).action,'close');
});
test('web resize grips send only their explicit native direction on primary press',()=>{
 const app=chrome();let prevented=0;
 for(const handle of app.handles){
  const before=app.calls.length;
  handle.listeners.pointerdown({button:2});assert.equal(app.calls.length,before);
  handle.listeners.pointerdown({button:0,preventDefault(){prevented++;}});
  assert.equal(app.calls.at(-1).action,`resize-${handle.dataset.direction}`);
 }
 assert.equal(prevented,8);
});

test('web page actions start disabled on Welcome and act on the trusted native child',async()=>{
 const start=chrome(); await Promise.resolve();
 assert.equal(start.nodes.get('copy').disabled,true);
 assert.equal(start.nodes.get('font-size').disabled,true);
 const page=chrome({welcome:false}); await Promise.resolve();
 assert.equal(page.nodes.get('copy').disabled,false);
 await page.nodes.get('copy').onclick();
 assert.equal(page.calls.at(-1).action,'copy-url');
 const size=page.nodes.get('font-size');
 for(const scale of [1.15,1.3,1.5,0.85,1]){
   await size.onclick();
   assert.deepEqual(page.calls.at(-1),{name:'web_chrome',action:'zoom',scale});
   assert.match(size.title,new RegExp(Math.round(scale*100)+'%'));
 }
});
test('a failed zoom keeps the current size and retries the same step',async()=>{
 const page=chrome({welcome:false,failZoom:true}); await Promise.resolve();
 const size=page.nodes.get('font-size');
 await size.onclick(); await size.onclick();
 assert.match(size.title,/100%/);
 assert.deepEqual(page.calls.filter(call=>call.action==='zoom').map(call=>call.scale),[1.15,1.15]);
});

test('reload dispatches one native request per click and skips overlapping requests', async () => {
 const start = chrome();
 await Promise.resolve();
 assert.equal(start.nodes.get('web-reload').disabled, true);

 const {nodes, calls} = chrome({welcome:false});
 await Promise.resolve();
 await nodes.get('web-back').onclick();
 assert.equal(calls.at(-1).action, 'back');

 const reload = nodes.get('web-reload');
 const pending = reload.onclick();
 await reload.onclick();
 await pending;
 assert.equal(calls.filter(call => call.action === 'reload').length, 1);
 await reload.onclick();
 assert.equal(calls.filter(call => call.action === 'reload').length, 2);
});
