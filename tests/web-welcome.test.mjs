import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {readFileSync} from 'node:fs';
function welcome({fail=false}={}) {
 const elements=Object.fromEntries(['web-form','web-url','run','status'].map(id=>[id,{value:'',disabled:false,textContent:'',attrs:{},setAttribute(k,v){this.attrs[k]=v;}}]));
 const calls=[];let resolve;
 const context={document:{getElementById:id=>elements[id]},window:{__TAURI__:{core:{invoke:async(name,args)=>{calls.push({name,args});if(fail)throw Error('URL 오류');return new Promise(r=>resolve=r);}}}}};
 vm.runInNewContext(readFileSync(new URL('../ui/web.js',import.meta.url),'utf8'),context);
 return {elements,calls,resolve:()=>resolve?.()};
}
test('web Welcome waits for explicit submit and sends the exact URL once',async()=>{
 const {elements,calls,resolve}=welcome();assert.equal(calls.length,0);
 elements['web-url'].value='https://example.com/path?q=hello#section';
 const event={preventDefault(){}};const first=elements['web-form'].onsubmit(event);await elements['web-form'].onsubmit(event);
 assert.equal(calls.length,1);assert.equal(calls[0].name,'web_run');assert.equal(calls[0].args.url,elements['web-url'].value);
 assert.equal(elements.run.disabled,true);assert.equal(elements['web-form'].attrs['aria-busy'],'true');resolve();await first;
});
test('invalid URL or native failure keeps input and allows retry without stealing focus',async()=>{
 const {elements,calls}=welcome({fail:true});elements['web-url'].value='javascript:alert(1)';
 await elements['web-form'].onsubmit({preventDefault(){}});
 assert.equal(elements.run.disabled,false);assert.match(elements.status.textContent,/URL 오류/);assert.equal(elements['web-url'].value,'javascript:alert(1)');
 assert.equal(elements['web-form'].attrs['aria-busy'],'false');await elements['web-form'].onsubmit({preventDefault(){}});assert.equal(calls.length,2);
});

test('web toolbar reflects pin state and closes the session instead of hiding it',async()=>{
 const elements=Object.fromEntries(['pin','close'].map(id=>[id,{attrs:{},handlers:{},title:'',setAttribute(k,v){this.attrs[k]=v;},addEventListener(name,fn){this.handlers[name]=fn;}}]));
 const calls=[];let pinned=true;
 const context={document:{getElementById:id=>elements[id]},window:{__TAURI__:{core:{invoke:async name=>{
  calls.push(name);
  if(name==='web_toolbar_info')return {always_on_top:true};
  if(name==='web_toggle_pin')return pinned=!pinned;
  if(name==='web_close')return {closed:true};
 }}}}};
 vm.runInNewContext(readFileSync(new URL('../ui/web-toolbar.js',import.meta.url),'utf8'),context);
 await Promise.resolve();
 assert.equal(elements.pin.attrs['aria-pressed'],'true');
 await elements.pin.handlers.click();
 assert.equal(elements.pin.attrs['aria-pressed'],'false');
 await elements.close.handlers.click();
 assert.deepEqual(calls,['web_toolbar_info','web_toggle_pin','web_close']);
});
