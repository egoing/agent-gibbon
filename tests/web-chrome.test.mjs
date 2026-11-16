import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {readFileSync} from 'node:fs';
function chrome() {
 const nodes=new Map(), calls=[];let alwaysOnTop=true;
 const node=id=>{
  if(!nodes.has(id)) nodes.set(id,{textContent:'',attributes:{},listeners:{},setAttribute(name,value){this.attributes[name]=value;},addEventListener(name,callback){this.listeners[name]=callback;}});
  return nodes.get(id);
 };
 const directions=['North','South','East','West','NorthWest','NorthEast','SouthWest','SouthEast'];
 const handles=directions.map(direction=>({...node(direction),dataset:{direction}}));
 vm.runInNewContext(readFileSync(new URL('../ui/web-chrome.js',import.meta.url),'utf8'),{
  document:{getElementById:node,querySelector:()=>node('main'),querySelectorAll:()=>handles},
  window:{__TAURI__:{core:{invoke:async(name,args)=>{calls.push({name,...args});if(args.action==='pin')alwaysOnTop=!alwaysOnTop;return {welcome:true,always_on_top:alwaysOnTop};}}}},
 });
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
