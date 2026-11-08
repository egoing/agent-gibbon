'use strict';
// Local installation handshake. No note content or input is sent to the updater.
(() => {
  const tauri = window.__TAURI__;
  if (!tauri?.core || !tauri.event) return;
  const raw = (window.gibbonInvoke || tauri.core.invoke).bind(tauri.core);
  const pending = new Map();
  const known = new Set(['snapshot', 'manager_notes', 'fit_note', 'save_note', 'export_note', 'terminal_info', 'poll_fold', 'poll_warp', 'present_feedback', 'feedback_snapshot']);
  let frozen = false;
  window.gibbonInvoke = (command, ...args) => {
    if (command.startsWith('updater_')) return raw(command, ...args);
    if (frozen) return Promise.reject('업데이트 설치를 준비하고 있습니다');
    const request = Symbol();
    pending.set(request, { kind: known.has(command) ? command : 'other', start: performance.now() });
    return Promise.resolve().then(() => raw(command, ...args)).finally(() => { pending.delete(request); });
  };
  for (const name of ['pointerdown', 'keydown', 'beforeinput', 'click', 'drop']) {
    window.addEventListener(name, event => {
      if (!frozen) return;
      event.preventDefault(); event.stopImmediatePropagation();
    }, true);
  }
  tauri.event.listen('updater-prepare', event => {
    frozen = true;
    const requests = {};
    const now = performance.now();
    for (const { kind, start } of pending.values()) {
      const bucket = requests[kind] ??= { count: 0, oldest_ms: 0 };
      bucket.count++;
      bucket.oldest_ms = Math.max(bucket.oldest_ms, Math.round(now - start));
    }
    const report = {
      editing: false, saving: false, exporting: false,
      ...window.gibbonUpdateActivity?.(),
      privacy: window.gibbonPrivacy?.updateBusy?.() === true,
      pending: pending.size,
      requests,
      oldest_ms: pending.size ? Math.round(now - pending.values().next().value.start) : 0,
    };
    const safe = pending.size === 0 && window.gibbonUpdateBusy?.() !== true && !report.privacy;
    raw('updater_ack', { epoch: event.payload, safe, report }).catch(() => { /* Missing ack blocks installation. */ });
  }).catch(() => {});
  tauri.event.listen('updater-release', () => { frozen = false; }).catch(() => {});
})();
