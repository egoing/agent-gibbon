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
