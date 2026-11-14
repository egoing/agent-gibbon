import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {readFileSync} from 'node:fs';
const source=readFileSync(new URL('../ui/relation-prototype.js',import.meta.url),'utf8');
function renderer(){
 let measured=0;
 const node=()=>({style:{},attributes:{},setAttribute(k,v){this.attributes[k]=v;}});
 const path=node(),text=node(),box=node(),label=node(),svg=node(),acks=[];
 text.getBBox=()=>{measured++;return{x:-20,y:-6,width:40,height:12};};
 const nodes={'.connector':path,text,rect:box,'.label':label};svg.querySelector=s=>nodes[s];
 const context={window:{__TAURI_INTERNALS__:{invoke:(command,args)=>{acks.push({command,...args});return Promise.resolve(true);}}},document:{querySelector:()=>svg},requestAnimationFrame(){throw Error('Hidden renderer must not schedule animation frames');}};
 vm.runInNewContext(source,context);
 return{window:context.window,path,label,acks,measured:()=>measured};
}
const payload={width:1000,height:800,scale:2,label:'참고',relation:['a','b'],points:[[0,0],[100,0],[200,200],[300,200]]};
const flush=()=>new Promise(resolve=>setImmediate(resolve));
test('settled geometry is drawn once and acknowledged without relying on a visible frame',async()=>{
 const r=renderer();r.window.drawRelation(payload,1);
 assert.equal(r.path.attributes.d,'M0,0 C100,0 200,200 300,200');assert.equal(r.acks.length,0);
 await flush();assert.deepEqual(r.acks,[{command:'relation_ready',generation:1}]);
 assert.equal(r.window.relationRenderStats.pending,false);assert.equal(r.measured(),1);
 const frames=r.window.relationRenderStats.frames;await flush();assert.equal(r.window.relationRenderStats.frames,frames);
});
test('newer settled geometry invalidates an old reveal and snaps to current endpoints',async()=>{
 const r=renderer();r.window.drawRelation(payload,1);
 r.window.drawRelation({...payload,points:[[10,20],[150,20],[250,240],[390,240]]},2);
 assert.equal(r.path.attributes.d,'M10,20 C150,20 250,240 390,240');
 await flush();assert.deepEqual(r.acks,[{command:'relation_ready',generation:2}]);assert.equal(r.measured(),1);
});
test('direction, identity and coordinate changes have no residual animation or repeated text measurement',async()=>{
 const r=renderer();r.window.drawRelation(payload,1);await flush();
 r.window.drawRelation({...payload,relation:['b','c'],scale:1,points:[[4,6],[8,6],[8,10],[12,10]]},2);
 assert.equal(r.path.attributes.d,'M4,6 C8,6 8,10 12,10');assert.equal(r.label.attributes.transform,'translate(8 8)');
 await flush();const frames=r.window.relationRenderStats.frames;
 r.window.drawRelation({...payload,relation:['b','c'],scale:1,points:[[4,6],[8,6],[8,10],[12,10]]},3);
 await flush();assert.equal(r.window.relationRenderStats.frames,frames);assert.equal(r.measured(),2);
 assert.equal(r.acks.at(-1).generation,3);
});
