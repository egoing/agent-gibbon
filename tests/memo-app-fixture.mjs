import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { memoThemeFor } from '../ui/theme.mjs';
import { readMemoAppearance } from '../ui/appearance.mjs';
import { feedbackView, durations } from '../ui/feedback-state.mjs';
export async function setup(initial = {}, state = {}, platform = 'MacIntel') {
  const nativeTargets = {};
  const elements = new Map(), calls = [], timers = [], events = {}, nativeEvents = {}, classes = new Set(), styles = {};
  const element = id => {
    if (!elements.has(id)) elements.set(id, { hidden: false, disabled: false, listeners: {}, attributes: {}, style: {},
      addEventListener(type, handler) { this.listeners[type] = handler; },
      setAttribute(k,v) { this.attributes[k]=v; }, removeAttribute(k) { delete this.attributes[k]; },
      getBoundingClientRect: () => ({ width: 298, height: id === 'controls' ? 22 : 120 }),
      cloneNode: () => ({ style: {}, querySelector: () => ({ hidden:false, innerHTML:'', remove() {} }), getBoundingClientRect: () => ({width:298,height:120}), remove() {} }),
    });
    return elements.get(id);
  };
  let note = { revision: 1, markdown: '# 원문', html: '<h1>원문</h1>', color: '#dfe2dd', ...initial };
  let copyError, exportError, warpError, acknowledgementError;
  let draft, base, dirty = false, composing = false, protectedSource = false, locked = false, onChange, error, saveDelay;
  const status = () => ({dirty, composing, protected:protectedSource,revision:base?.revision});
  const adapter = {
    status, sync(next) { if (dirty || composing || locked) return {...status(),conflict:base.revision !== next.revision}; base={...next};draft=next.markdown;return status(); },
    discard(next) { base={...next};draft=next.markdown;dirty=false; },
    markdown: () => draft,
    capture: () => ({markdown:draft,revision:base.revision}),
    pending() {},
    saved(next, snapshot) { base={...next};dirty=draft!==snapshot.markdown;onChange(status()); },
    lock(value) {locked=value;}, highlight(html) {element('editor').highlight=html;}, focus() {calls.push(['editor-focus']);},
  };
  const context = vm.createContext({
    navigator:{platform}, memoThemeFor,readMemoAppearance,feedbackView,durations,
    createMemoEditor:async (_root,fn)=>{onChange=fn;return adapter;}, createFeedback:()=>()=>{},
    clearTimeout(fn) {const i=timers.indexOf(fn);if(i>=0)timers.splice(i,1);},setTimeout(fn){timers.push(fn);return fn;},
    document:{getElementById:element,querySelectorAll:()=>[],addEventListener(type,fn){events[type]=fn;},
      body:{appendChild(){},classList:{toggle(k,v){v?classes.add(k):classes.delete(k);}}},
      createElement:()=>({style:{},setAttribute(){},remove(){},get innerText(){return this.innerHTML.replace(/<[^>]+>/g,'').trim();}}),
      documentElement:{style:{setProperty(k,v){styles[k]=v;}},addEventListener(){}},
    },
    window:{getSelection:()=>({toString:()=>state.selection || ''}),__TAURI__:{
      clipboardManager:{async writeText(text){if(copyError)throw copyError;calls.push(['clipboard-text',text]);}},
      window:{getCurrentWindow:()=>({label:'note-fixture',startResizeDragging:async()=>{}})},
      event:{async listen(name,fn,options){nativeEvents[name]=fn;nativeTargets[name]=options?.target;return ()=>{};}},
      core:{async invoke(name,args){
        calls.push([name,args]);
        if(name==='snapshot')return {...state,note:{...note},always_on_top:true};
        if(name==='fit_note')return true;
        if(name==='poll_fold')return !!state.folded;
        if(name==='toggle_fold')return state.folded=!state.folded;
        if(name==='acknowledge_feedback'){if(acknowledgementError)throw acknowledgementError;return note.revision===args.revision;}
        if(name==='export_note'){if(exportError)throw exportError;return '/tmp/export.md';}
        if(name==='warp_note'&&warpError)throw warpError;
        if(name==='save_note'){
          if(saveDelay)await saveDelay;
          if(error)throw error;
          if(note.revision!==args.expectedRevision)throw new Error('메모가 외부에서 갱신되었습니다');
          note={...note,markdown:args.markdown,html:'<p>저장됨</p>',revision:note.revision+1};return {...note};
        }
      }},
    }},
  });
  vm.runInContext(readFileSync(new URL('../ui/app.js',import.meta.url),'utf8').replace(/^import .*;$/gm,''),context);
  const settle=()=>new Promise(setImmediate);
  await settle();
  const refresh=async()=>{await timers.find(fn=>fn.name==='refresh')();await settle();};
  return {element,calls,events,nativeEvents,nativeTargets,classes,styles,adapter,state,
    failCopy(value){copyError=value;},failExport(value){exportError=value;},failWarp(value){warpError=value;},failAcknowledgement(value){acknowledgementError=value;},
    flushAutosave:async()=>{const fn=timers.find(fn=>fn.name==='autosaveTick');if(fn){timers.splice(timers.indexOf(fn),1);await fn();await settle();}},
    refresh,settle, note:()=>note, draft:()=>draft,locked:()=>locked,
    input(value){draft=value;dirty=value!==base.markdown;element('editor').innerText=value;onChange(status());},
    composition(value){composing=value;onChange(status());},
    protect(value){protectedSource=value;onChange(status());},
    update(patch){note={...note,...patch};}, failSave(value){error=value;}, delaySave(value){saveDelay=value;},
    action:async action=>{await nativeEvents['feedback-action']({payload:{owner:'note-fixture',action}});await settle();},
    key:async(key,props={})=>{events.keydown({key,preventDefault(){},...props});await settle();},
    view:()=>calls.filter(([name])=>name==='present_feedback').at(-1)?.[1].view,
    busy:()=>context.window.gibbonUpdateBusy(),
    leave:async()=>{state.focused=true;await refresh();state.focused=false;await refresh();},
    pollFold:async()=>{await timers.find(fn=>fn.name==='pollFold')();await settle();},
  };
}
