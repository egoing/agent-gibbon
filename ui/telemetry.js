'use strict';
// Only fixed operation names + bounded numbers cross IPC. No content is read.
(() => {
  const core = window.__TAURI__?.core;
  if (!core) return;
  const raw = core.invoke.bind(core);
  let active=false, generation=0, inFlight=false, interactionPending=false;
  const sources = new Set(['app.js','telemetry.js','terminal.js','terminal-ime.js','toolbar.js','manager.js','privacy.js','feedback.js','capture.js','stash.js']);
  for (const kind of ['error', 'unhandledrejection']) {
    window.addEventListener(kind, event => {
      if(!active)return;
      const data = {kind};
      if (kind === 'error') {
        const source = String(event.filename || '').split(/[\\/]/).pop();
        if (sources.has(source)) {
          data.source = source;
          for (const [key, value] of [['line', event.lineno], ['column', event.colno]]) {
            if (Number.isInteger(value) && value >= 0 && value <= 10000000) data[key] = value;
          }
        }
      }
      raw('report_frontend_error', data).catch(() => {});
    });
  }
  const operations = new Map([
    ['snapshot', 'snapshot'], ['save_note', 'save_note'], ['export_note', 'export_note'],
    ['toggle_pin', 'toggle_pin'], ['hide_note', 'hide_note'], ['close_note', 'close_note'],
    ['warp_note', 'warp_note'], ['toggle_fold', 'toggle_fold'], ['capture_screen', 'capture_screen'],
    ['finish_capture', 'finish_capture'], ['copy_note_image', 'copy_note_image'],
    ['manager_notes', 'manager_notes'], ['manager_image', 'manager_image'], ['manager_terminal', 'manager_terminal'],
    ['terminal_start', 'terminal.start'], ['terminal_write', 'terminal.write'], ['terminal_resize', 'terminal.resize'],
    ['terminal_hide', 'terminal.hide'], ['terminal_info', 'terminal.info'], ['terminal_welcome', 'terminal.welcome'],
    ['terminal_folder', 'terminal.folder'], ['terminal_rename', 'terminal.rename'], ['terminal_close', 'terminal.close'],
  ]);
  const queue=new Map(), timers=[], observers=[];
  const now=()=>window.performance.now();
  function record(op,ms,failed=false) {
    if(!active || !Number.isFinite(ms) || ms<0 || ms>600000)return;
    const item=queue.get(op)||{op,duration_ms:0,count:0,failure_count:0};
    item.duration_ms=Math.max(item.duration_ms,ms);
    if(item.count<10000){item.count++;if(failed)item.failure_count++;}
    if(queue.size<32 || queue.has(op))queue.set(op,item);
  }
  window.gibbonInvoke=(command,...args)=>{
    const op=operations.get(command);
    if(!active || !op)return raw(command,...args);
    const started=now(),session=generation;
    return raw(command,...args).then(value=>{if(session===generation)record(op,now()-started);return value;},
      error=>{if(session===generation)record(op,now()-started,true);throw error;});
  };
  window.addEventListener('click',()=>{
    if(!active || interactionPending || window.document.visibilityState!=='visible')return;
    interactionPending=true;const start=now(),session=generation;
    window.requestAnimationFrame(()=>window.requestAnimationFrame(()=>{
      if(session!==generation)return;
      interactionPending=false;
      if(window.document.visibilityState==='visible')record('webview.interaction',now()-start);
    }));
  },{passive:true});
  function configure(config) {
    const enabled=config?.detailed===true;
    if(active===enabled)return;
    active=enabled;generation++;inFlight=false;interactionPending=false;queue.clear();
    for(const id of timers)window.clearInterval?.(id);timers.length=0;
    for(const observer of observers)observer.disconnect?.();observers.length=0;
    delete window.gibbonMeasure;
    if(!active)return;
    const session=generation;
    const stages=new Set(['terminal.ui.ready','terminal.first_output','terminal.output.parse']);
    window.gibbonMeasure=(op,duration)=>{
      if(session===generation && stages.has(op) && window.document.visibilityState==='visible')record(op,duration);
    };
    function beat() {
      if(!active || session!==generation || inFlight)return;
      inFlight=true;const measurements=[...queue.values()];queue.clear();
      raw('telemetry_heartbeat',{measurements}).catch(()=>{}).finally(()=>{if(session===generation)inFlight=false;});
    }
    const Observer=window.PerformanceObserver;
    for(const [type,op] of [['longtask','webview.long_task'],['long-animation-frame','webview.long_frame']]){
      if(!Observer?.supportedEntryTypes?.includes(type))continue;
      try{
        const observer=new Observer(list=>{if(session===generation)for(const entry of list.getEntries())record(op,entry.duration);});
        observer.observe({type,buffered:false});observers.push(observer);
      }catch{}
    }
    let last=now();
    timers.push(window.setInterval(()=>{
      if(!active || session!==generation)return;
      const time=now(),elapsed=time-last;last=time;
      if(window.document.visibilityState==='visible' && elapsed<60000 && elapsed>1250)record('webview.event_loop_lag',elapsed-1000);
    },1000));
    timers.push(window.setInterval(beat,2000));beat();
  }
  window.__TAURI__.event?.listen('privacy-changed',event=>configure(event.payload)).catch(()=>{});
  raw('telemetry_status').then(configure).catch(()=>{});
})();
