'use strict';
// Local installation handshake. No note content or input is sent to the updater.
(() => {
  const tauri = window.__TAURI__;
  if (!tauri?.core || !tauri.event) return;
  const raw = (window.gibbonInvoke || tauri.core.invoke).bind(tauri.core);
  let pending = 0, frozen = false;
  window.gibbonInvoke = (command, ...args) => {
    if (command.startsWith('updater_')) return raw(command, ...args);
    if (frozen) return Promise.reject('업데이트 설치를 준비하고 있습니다');
    pending++;
    return Promise.resolve().then(() => raw(command, ...args)).finally(() => { pending--; });
  };
  for (const name of ['pointerdown', 'keydown', 'beforeinput', 'click', 'drop']) {
    window.addEventListener(name, event => {
      if (!frozen) return;
      event.preventDefault(); event.stopImmediatePropagation();
    }, true);
  }
  tauri.event.listen('updater-prepare', event => {
    frozen = true;
    const safe = pending === 0 && window.gibbonUpdateBusy?.() !== true && window.gibbonPrivacy?.updateBusy?.() !== true;
    raw('updater_ack', { epoch: event.payload, safe }).catch(() => { /* Missing ack blocks installation. */ });
  }).catch(() => {});
  tauri.event.listen('updater-release', () => { frozen = false; }).catch(() => {});
})();
