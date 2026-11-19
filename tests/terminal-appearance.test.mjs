import test from 'node:test';
import assert from 'node:assert/strict';
import {installTerminalAppearance} from '../ui/terminal-appearance.mjs';
import {memoDesignKey} from '../ui/appearance.mjs';
function setup(design='gray',{failRead=false,dark=false}={}){
 const values=new Map([[memoDesignKey,design]]),styles=new Map(),events={};
 const storage={getItem(key){if(failRead)throw Error('blocked');return values.get(key);}};
 const media={matches:dark,addEventListener(name,handler){this.change=handler;}};
 const env={localStorage:storage,matchMedia:()=>media,addEventListener(name,handler){events[name]=handler;}};
 const root={style:{setProperty:(key,value)=>styles.set(key,value)}};
 installTerminalAppearance(env,root);
 return {styles,root,media,env,storage,change(design){values.set(memoDesignKey,design);events.storage({key:memoDesignKey,storageArea:storage});},event:event=>events.storage(event)};
}
test('terminal Welcome starts with the saved independent design and a safe gray fallback',()=>{
 for(const [design,bg] of [['white','#f8f8f8'],['gray','#dfe2dd'],['black','#262627'],['color','#20252e'],['invalid','#dfe2dd']])assert.equal(setup(design).styles.get('--background'),bg);
 assert.equal(setup('black',{failRead:true}).styles.get('--background'),'#dfe2dd');
});
test('cross-document changes update color roles and scheme without invoking terminal state',()=>{
 const app=setup();app.change('black');assert.equal(app.styles.get('--background'),'#262627');assert.equal(app.root.style.colorScheme,'dark');
 app.change('white');assert.equal(app.styles.get('--foreground'),'#2d2d2d');assert.equal(app.root.style.colorScheme,'light');
 app.event({key:'jajan.manager-theme',storageArea:app.storage});assert.equal(app.styles.get('--background'),'#f8f8f8');
 app.event({key:memoDesignKey,storageArea:{getItem(){return 'black';}}});assert.equal(app.styles.get('--background'),'#f8f8f8');
 app.env.localStorage.getItem=()=>null;app.event({key:null,storageArea:app.storage});assert.equal(app.styles.get('--background'),'#dfe2dd');
});
test('only system Welcome follows OS light/dark; explicit gray remains fixed',()=>{
 const app=setup('system');app.media.matches=true;app.media.change();assert.equal(app.styles.get('--background'),'#262627');
 app.change('gray');app.media.matches=false;app.media.change();assert.equal(app.styles.get('--background'),'#dfe2dd');
 app.media.matches=true;app.media.change();assert.equal(app.styles.get('--background'),'#dfe2dd');
});

test('trusted chrome loads stored color and receives updates while preserving other designs', async()=>{
 const styles=new Map(),events={},storage={getItem:()=> 'color'};
 let listener;
 const env={localStorage:storage,location:{pathname:'/web.html'},matchMedia:()=>({addEventListener(){}}),addEventListener:(key,fn)=>events[key]=fn,__TAURI__:{event:{listen:async(key,fn)=>{listener=fn;}},core:{invoke:async(name,args)=>{assert.equal(name,'web_chrome');assert.equal(args.action,'inspect');return {color:'#db704f'};}}}};
 const root={dataset:{},style:{setProperty:(k,v)=>styles.set(k,v)}};
 installTerminalAppearance(env,root);await new Promise(resolve=>setImmediate(resolve));
 assert.equal(styles.get('--background'),'#db704f');assert.equal(styles.get('--foreground'),'#291b16');
 listener({payload:'#4d4d4b'});assert.equal(styles.get('--background'),'#4d4d4b');
 env.localStorage.getItem=()=> 'gray';events.storage({key:memoDesignKey,storageArea:storage});
 listener({payload:'#6baa92'});assert.equal(styles.get('--background'),'#dfe2dd');
 env.localStorage.getItem=()=> 'color';events.storage({key:memoDesignKey,storageArea:storage});assert.equal(styles.get('--background'),'#6baa92');
});
