import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {readFileSync} from 'node:fs';
function chrome() {
 const nodes=new Map(), calls=[];
 const node=id=>{
  if(!nodes.has(id)) nodes.set(id,{textContent:'',listeners:{},setAttribute(){},addEventListener(name,callback){this.listeners[name]=callback;}});
  return nodes.get(id);
 };
 const directions=['North','South','East','West','NorthWest','NorthEast','SouthWest','SouthEast'];
 const handles=directions.map(direction=>({...node(direction),dataset:{direction}}));
 vm.runInNewContext(readFileSync(new URL('../ui/web-chrome.js',import.meta.url),'utf8'),{
  document:{getElementById:node,querySelector:()=>node('main'),querySelectorAll:()=>handles},
  window:{__TAURI__:{core:{invoke:async(name,args)=>{calls.push({name,...args});return {welcome:true,always_on_top:true};}}}},
 });
 return {nodes,calls,handles};
}
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
