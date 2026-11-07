'use strict';
(() => {
  const invoke = window.gibbonInvoke || window.__TAURI__.core.invoke;
  const el = id => document.getElementById(id);
  let state = null, busy = false;
  const api = window.gibbonPrivacy = {
    accepted: () => Boolean(state?.terms_accepted || state?.terms_skipped),
    onAccepted: null,
    updateBusy: () => busy || !state || el("privacy-setting-share").checked !== Boolean(state.allowed ?? state.sharing),
    showWelcome: () => { el('manager-navigation').hidden=true; el('privacy-welcome').hidden=false; el('notes-page').hidden=true; el('settings-page').hidden=true; },
    refresh,
  };
  function error(message) { el('privacy-error').textContent=message; el('privacy-error').hidden=!message; el('privacy-retry').hidden=!message; }
  function render() {
    const accepted=api.accepted(), ready=Boolean(state?.notice_ready);
    el('privacy-welcome').hidden=accepted;
    el('manager-navigation').hidden=!accepted;
    if(!busy){
      el('privacy-terms').checked=Boolean(state?.terms_accepted);
      el('privacy-share').checked=Boolean(state?.allowed ?? state?.sharing);
      el('privacy-setting-share').checked=Boolean(state?.allowed ?? state?.sharing);
    }
    el('privacy-share').disabled=busy || !ready;
    el('privacy-share').setAttribute('aria-describedby',ready?'':'privacy-draft');
    el('privacy-setting-share').disabled=busy || !ready;
    el('privacy-continue').disabled=busy || !state || !el('privacy-terms').checked;
    el('privacy-save').disabled=busy || !state;
    el('privacy-terms').disabled=busy || !state;
    for(const id of ['privacy-draft','privacy-settings-draft']) el(id).hidden=ready;
    el('privacy-sharing-status').textContent=state?.sharing?'Sharing enabled':'Not sharing';
    el('privacy-storage-warning').hidden=!state?.load_failed;
    if(!accepted) api.showWelcome();
  }
  async function refresh() {
    try { state=await invoke('privacy_status'); error(''); render(); return state; }
    catch { state=null; render(); error('Could not load privacy choices. Sharing stays off. Retry to continue.'); throw Error('Privacy choices unavailable'); }
  }
  async function save(onboarding) {
    if(busy || !state) return;
    if(onboarding && !el('privacy-terms').checked) return;
    const sharing=el(onboarding?'privacy-share':'privacy-setting-share').checked;
    // Development access is not a recorded agreement. Ask explicitly before
    // saving a first diagnostic choice, even when the welcome gate was skipped.
    if(!onboarding && !state.terms_accepted) {
      if(sharing) {
        api.showWelcome();
        el('privacy-share').checked=true;
      } else el('privacy-save-result').textContent='Not sharing. No changes to save.';
      return;
    }
    busy=true; render(); error('');
    try {
      state=await invoke('privacy_save',{termsAccepted:true,shareDiagnostics:sharing});
      render();
      if(onboarding) api.onAccepted?.();
      else el('privacy-save-result').textContent='Saved. Your choice applies now.';
    } catch(e) { error(String(e)); }
    finally { busy=false; render(); }
  }
  el('privacy-terms').addEventListener('change',()=>{el('privacy-continue').disabled=busy || !el('privacy-terms').checked;});
  el('privacy-continue').onclick=()=>save(true);
  el('privacy-save').onclick=()=>save(false);
  el('privacy-setting-share').addEventListener('change',()=>{if(!el('privacy-setting-share').checked)save(false);});
  el('privacy-exit').onclick=()=>invoke('privacy_quit').catch(()=>error('Could not exit. Please close the app.'));
  el('privacy-retry').onclick=()=>refresh().catch(()=>{});
  for(const id of ['privacy-read-terms','privacy-settings-terms','privacy-read-notice','privacy-settings-notice']) {
    el(id).onclick=()=>{
      if(!state) return;
      el('privacy-document-title').textContent=id.includes('terms')?'Terms of Use':'Privacy Notice';
      el('privacy-document-content').innerHTML=id.includes('terms')?state.terms_html:state.notice_html;
      el('privacy-document').hidden=false; el('privacy-document').focus();
    };
  }
  el('privacy-document-close').onclick=()=>{el('privacy-document').hidden=true;};
  el('privacy-document-content').addEventListener('click',e=>{if(e.target.closest('a'))e.preventDefault();});
  api.ready=refresh();
  api.ready.catch(()=>{});
})();
