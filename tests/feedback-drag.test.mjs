import test from 'node:test';
import assert from 'node:assert/strict';
import { createFeedback } from '../ui/feedback.js';

function fixture() {
  const make = () => ({ hidden: false, dataset: {}, children: [], listeners: {}, attributes: {},
    classList: {toggle() {}}, setAttribute(k,v) {this.attributes[k]=v;},
    addEventListener(k,v) {this.listeners[k]=v;}, appendChild(b) {this.children.push(b);b.remove=()=>{this.children=this.children.filter(x=>x!==b);};} });
  const elements = Object.fromEntries(['feedback','message','actions'].map(id=>[id,make()]));
  const calls=[];
  const render=createFeedback({getElementById:id=>elements[id],createElement:make,defaultView:{navigator:{platform:'MacIntel'}}},(...args)=>calls.push(args));
  return {elements,calls,render};
}
test('inline footer preserves action focus identities and uses the latest revision', () => {
  const {render,elements,calls}=fixture();
  const view={message:'',revision:1,buttons:[{id:'save',label:'메모에 반영',disabled:false}]};
  render(view);
  const button=elements.actions.children[0];
  render({...view,revision:2,message:'원본 변경',error:true});
  assert.equal(elements.actions.children[0],button);
  assert.equal(elements.message.attributes.role,'alert');
  button.listeners.click();assert.deepEqual(calls,[['save',2]]);
  assert.match(button.attributes['aria-label'],/Command S/);
  render({...view,buttons:[]});assert.equal(elements.feedback.hidden,true);
  assert.equal(elements.actions.children.length,0);
});
test('fold and hidden memo suppression do not discard the view or its buttons', () => {
  const {render,elements}=fixture();
  const view={message:'안내',revision:1,buttons:[{id:'cancel',label:'취소'}]};
  render(view,true);assert.equal(elements.feedback.hidden,true);
  const button=elements.actions.children[0];
  render(view,false);assert.equal(elements.feedback.hidden,false);
  assert.equal(elements.actions.children[0],button);
});
