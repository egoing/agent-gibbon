import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {readFileSync} from 'node:fs';
const geometry=readFileSync(new URL('../ui/relation-curve.js',import.meta.url),'utf8');
const source=readFileSync(new URL('../ui/relation-prototype.js',import.meta.url),'utf8');
function renderer(){
 let measured=0;
 const node=()=>({style:{},attributes:{},setAttribute(k,v){this.attributes[k]=v;}});
 const path=node(),text=node(),box=node(),label=node(),svg=node(),marker=node(),acks=[];
 text.getBBox=()=>{measured++;return{x:-20,y:-6,width:40,height:12};};
 const nodes={'.connector':path,marker,text,rect:box,'.label':label};svg.querySelector=s=>nodes[s];
 const context={window:{__TAURI_INTERNALS__:{invoke:(command,args)=>{acks.push({command,...args});return Promise.resolve(true);}}},document:{querySelector:()=>svg},requestAnimationFrame(){throw Error('Hidden renderer must not schedule animation frames');}};
 vm.runInNewContext(geometry,context);
 vm.runInNewContext(source,context);
 return{window:context.window,path,label,marker,acks,measured:()=>measured};
}
const payload={width:1000,height:800,scale:2,arrow:'none',label:'참고',relation:['a','b'],points:[[0,0],[100,0],[200,200],[300,200]]};
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
test('arrow choice supports target-only, both ends and no arrow without stale markers',async()=>{
 const r=renderer();
 for(const [arrow,start,end] of [['both','url(#arrow)','url(#arrow)'],['none','none','none'],['end','none','url(#arrow)']]){
  r.window.drawRelation({...payload,arrow},1);
  assert.equal(r.path.attributes['marker-start'],start);
  assert.equal(r.path.attributes['marker-end'],end);
 }
 await flush();
});

test('a steep diagonal joins the center of the full-size head base and keeps its tip at the port',()=>{
 const r=renderer(), points=[[300,145],[300,122.5],[120,122.5],[120,100]];
 r.window.drawRelation({...payload,scale:1,points,arrow:'end'},1);
 assert.equal(r.path.attributes.d,'M300,145 C300,129.5 120,129.5 120,114');
 assert.equal(r.marker.attributes.markerHeight,7);
 assert.equal(r.label.attributes.transform,'translate(210 129.5)');
 for(const name of ['relation-prototype','relation-pick']){
  const html=readFileSync(new URL(`../ui/${name}.html`,import.meta.url),'utf8');
  assert.match(html,/<marker [^>]*refX="0" refY="5"/);
 }
});
test('opposed heads fit inside a narrow gap and reset to full size after movement',()=>{
 const r=renderer(), points=[[0,20],[4,20],[4,200],[8,200]];
 r.window.drawRelation({...payload,scale:1,points,arrow:'both'},1);
 assert.equal(r.path.attributes.d,'M3.5,20 C4,20 4,200 4.5,200');
 assert.equal(r.marker.attributes.markerWidth,1.75);
 r.window.drawRelation({...payload,scale:1,points:[[0,20],[20,20],[20,200],[40,200]],arrow:'both'},2);
 assert.equal(r.marker.attributes.markerWidth,7);
 assert.equal(r.path.attributes.d,'M14,20 C20,20 20,200 26,200');
});
test('touching ports retain the straight chord and coincident ports have no drawable arrow',()=>{
 const r=renderer(), points=[[20,10],[20,10],[20,80],[20,80]];
 r.window.drawRelation({...payload,scale:1,points,arrow:'end'},1);
 assert.equal(r.path.attributes.d,'M20,10 C20,38 20,38 20,66');
 r.window.drawRelation({...payload,points:Array(4).fill([20,10]),arrow:'both'},2);
 assert.equal(r.path.attributes.d,'');
});
