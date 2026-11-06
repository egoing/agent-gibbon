import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {readFileSync} from 'node:fs';
const source=readFileSync(new URL('../ui/privacy.js',import.meta.url),'utf8');
const html=readFileSync(new URL('../ui/manager.html',import.meta.url),'utf8');
const tick=()=>new Promise(setImmediate);
function setup(initial={terms_accepted:false,sharing:false,notice_ready:true},fail=false){
 const elements=new Map();
 for(const [,id] of html.matchAll(/\bid="([^"]+)"/g))elements.set(id,{hidden:true,checked:false,disabled:true,textContent:'',innerHTML:'',listeners:{},addEventListener(k,fn){this.listeners[k]=fn;},focus(){this.focused=true;}});
 const calls=[];let state={terms_html:'<h1>Terms</h1>',notice_html:'<h1>Privacy</h1>',...initial};
 const window={__TAURI__:{core:{invoke:async(name,args)=>{
   calls.push({name,args});
   if(name==='privacy_status'){if(fail)throw Error('disk failure');return state;}
   if(name==='privacy_save'){
    if(!args.termsAccepted)throw Error('terms required');
    if(args.shareDiagnostics&&!state.notice_ready)throw Error('Sharing unavailable');
    state={...state,terms_accepted:true,sharing:args.shareDiagnostics};return state;
   }
 }}}};
 vm.runInNewContext(source,{window,document:{getElementById:id=>elements.get(id)},Boolean,Error});
 return{window,elements,calls};
}
test('required terms do not imply optional sharing; unchecked Continue submits nothing',async()=>{
 const s=setup();await tick();const e=s.elements;
 assert.equal(e.get('privacy-terms').checked,false);assert.equal(e.get('privacy-share').checked,false);assert.equal(e.get('privacy-continue').disabled,true);
 await e.get('privacy-continue').onclick();assert.equal(s.calls.filter(c=>c.name==='privacy_save').length,0);
 e.get('privacy-terms').checked=true;e.get('privacy-terms').listeners.change();assert.equal(e.get('privacy-continue').disabled,false);
 let accepted=0;s.window.gibbonPrivacy.onAccepted=()=>accepted++;
 await e.get('privacy-continue').onclick();assert.equal(accepted,1);
 const write=s.calls.find(c=>c.name==='privacy_save');assert.deepEqual(JSON.parse(JSON.stringify(write.args)),{termsAccepted:true,shareDiagnostics:false});
 assert.equal(s.window.gibbonPrivacy.accepted(),true);
});
test('unfinished notice disables optional sharing but still allows terms-only use',async()=>{
 const s=setup({terms_accepted:false,sharing:false,notice_ready:false});await tick();
 assert.equal(s.elements.get('privacy-share').disabled,true);assert.equal(s.elements.get('privacy-draft').hidden,false);
 s.elements.get('privacy-terms').checked=true;await s.elements.get('privacy-continue').onclick();
 assert.equal(s.window.gibbonPrivacy.accepted(),true);assert.equal(s.calls.find(c=>c.name==='privacy_save').args.shareDiagnostics,false);
});
test('stored decline stays declined and settings withdrawal is an explicit false',async()=>{
 const s=setup({terms_accepted:true,sharing:true,notice_ready:true});await tick();
 s.elements.get('privacy-setting-share').checked=false;await s.elements.get('privacy-save').onclick();
 assert.equal(s.calls.find(c=>c.name==='privacy_save').args.shareDiagnostics,false);
 assert.equal(s.elements.get('privacy-setting-share').checked,false);assert.match(s.elements.get('privacy-save-result').textContent,/applies now/);
});
test('unreadable status fails closed; legal documents remain local',async()=>{
 const failed=setup(undefined,true);await tick();assert.equal(failed.elements.get('privacy-continue').disabled,true);assert.equal(failed.window.gibbonPrivacy.accepted(),false);
 const s=setup();await tick();s.elements.get('privacy-read-terms').onclick();assert.equal(s.elements.get('privacy-document-content').innerHTML,'<h1>Terms</h1>');
 let blocked=false;s.elements.get('privacy-document-content').listeners.click({target:{closest:()=>true},preventDefault(){blocked=true;}});assert.equal(blocked,true);
 assert.deepEqual(s.calls.map(c=>c.name),['privacy_status']);
});
test('turning sharing off submits withdrawal without another Save click',async()=>{
 const s=setup({terms_accepted:true,sharing:true,notice_ready:true});await tick();
 s.elements.get('privacy-setting-share').checked=false;s.elements.get('privacy-setting-share').listeners.change();await tick();
 assert.equal(s.calls.find(c=>c.name==='privacy_save').args.shareDiagnostics,false);
});
