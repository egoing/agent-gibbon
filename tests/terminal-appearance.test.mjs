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
 app.event({key:'agent-gibbon.manager-theme',storageArea:app.storage});assert.equal(app.styles.get('--background'),'#f8f8f8');
 app.event({key:memoDesignKey,storageArea:{getItem(){return 'black';}}});assert.equal(app.styles.get('--background'),'#f8f8f8');
 app.env.localStorage.getItem=()=>null;app.event({key:null,storageArea:app.storage});assert.equal(app.styles.get('--background'),'#dfe2dd');
});
test('only system Welcome follows OS light/dark; explicit gray remains fixed',()=>{
 const app=setup('system');app.media.matches=true;app.media.change();assert.equal(app.styles.get('--background'),'#262627');
 app.change('gray');app.media.matches=false;app.media.change();assert.equal(app.styles.get('--background'),'#dfe2dd');
 app.media.matches=true;app.media.change();assert.equal(app.styles.get('--background'),'#dfe2dd');
});
