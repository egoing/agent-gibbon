'use strict';
(() => {
  const invoke = window.jajanInvoke || window.__TAURI__.core.invoke;
  const el = id => document.getElementById(id);
  let state = null, busy = false, pending = null, welcomeDirty = false, version = 0, readFailed = false;
  function error(value = '') { window.jajanLanguage.bind(el('autostart-error'), 'textContent', value); el('autostart-error').hidden=!value; }
  function render() {
    const available = Boolean(state?.available);
    el('autostart-setting').disabled = busy || !available;
    el('autostart-retry').hidden = !readFailed;
    el('autostart-retry').disabled = busy;
    el('autostart-os-settings').hidden = !['requires_approval','registered_unknown'].includes(state?.os_status);
    el('autostart-os-settings').disabled = busy;
    el('autostart-welcome').disabled = busy || !available;
    el('autostart-setting').checked = pending ?? (state?.selected === true);
    if (!welcomeDirty) el('autostart-welcome').checked = state?.selected ?? true;
    window.jajanLanguage.bind(el('autostart-welcome-status'), 'textContent', available ? (state.selected == null ? 'Continue를 눌러 확인하기 전에는 등록하지 않습니다.' : '저장된 앱 선택입니다. Continue를 눌러 현재 선택을 확인합니다.') : (state?.reason || '상태를 확인하지 못했습니다. Continue 이후 설정에서 선택할 수 있습니다.'));
  }
  async function refresh() {
    if(busy) return;
    const ticket=++version;
    try { const next = await invoke('autostart_status'); if(ticket!==version || busy) return; state=next; readFailed=false; error(); }
    catch(e) { if(ticket!==version || busy) return; state=null; readFailed=true; error(String(e)); }
    render();
  }
  async function save(enabled) {
    if(busy || !state?.available) return;
    busy=true; pending=enabled; ++version; render(); error();
    try {
      state=await invoke('autostart_save',{enabled}); welcomeDirty=false; readFailed=false;
      return true;
    } catch(e) {
      error(window.jajanLanguage.msg('{error} 앱 선택과 OS 상태가 다를 수 있습니다. 아래 상태를 확인하세요.', {error:window.jajanLanguage.msg(String(e))}));
      // Read after failure; never retry OS registration automatically.
      try { state=await invoke('autostart_status'); readFailed=false; } catch { state=null; readFailed=true; }
      return false;
    } finally { busy=false; pending=null; render(); }
  }
  const ready=refresh();
  window.jajanAutostart = {
    refresh,
    updateBusy: () => busy,
    async confirmWelcome() {
      await ready;
      // Debug/unavailable is never an implicit agreement or OS registration.
      if(state?.available && !await save(el('autostart-welcome').checked)) throw Error('자동 실행 설정을 적용하지 못했습니다. 설정의 로그인 자동 실행에서 앱 선택과 OS 상태를 확인하세요.');
    },
  };
  el('autostart-setting').addEventListener('change',()=>save(el('autostart-setting').checked));
  el('autostart-welcome').addEventListener('change',()=>{welcomeDirty=true;});
  el('autostart-retry').onclick=refresh;
  el('autostart-os-settings').onclick=()=>invoke('autostart_open_settings').catch(e=>error(String(e)));
  window.addEventListener('focus',()=>{ if(!el('settings-autostart').hidden) refresh(); });
})();
