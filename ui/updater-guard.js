'use strict';
// Local installation handshake. No note content or input is sent to the updater.
(() => {
  const tauri = window.__TAURI__;
  if (!tauri?.core || !tauri.event) return;
  const raw = (window.gibbonInvoke || tauri.core.invoke).bind(tauri.core);
  const pending = new Map();
  const known = new Set(['snapshot', 'manager_notes', 'fit_note', 'save_note', 'export_note', 'terminal_info', 'poll_fold', 'poll_warp', 'present_feedback', 'feedback_snapshot']);
  const background = new Set(['snapshot', 'poll_warp', 'poll_fold', 'feedback_snapshot', 'manager_notes', 'terminal_info']);
  const drainMs = 500; // Native waits up to three seconds for every window.
  let latestEpoch = 0;
  let frozenEpoch = null;
  let acknowledgeDrained = null;
  let drainTimer;

  function cancelDrain() {
    clearTimeout(drainTimer);
    acknowledgeDrained = null;
  }
  window.gibbonInvoke = (command, ...args) => {
    if (command.startsWith('updater_')) return raw(command, ...args);
    if (frozenEpoch !== null) return Promise.reject('업데이트 설치를 준비하고 있습니다');
    const request = Symbol();
    pending.set(request, { kind: known.has(command) ? command : 'other', start: performance.now() });
    return Promise.resolve().then(() => raw(command, ...args)).finally(() => {
      pending.delete(request);
      if (!pending.size && acknowledgeDrained) {
        clearTimeout(drainTimer);
        // Let the command's caller finish processing the result before reporting.
        drainTimer = setTimeout(acknowledgeDrained, 0);
      }
    });
  };
  for (const name of ['pointerdown', 'keydown', 'beforeinput', 'click', 'drop']) {
    window.addEventListener(name, event => {
      if (frozenEpoch === null) return;
      event.preventDefault(); event.stopImmediatePropagation();
    }, true);
  }
  function activity() {
    return {
      editing: false, saving: false, exporting: false,
      ...window.gibbonUpdateActivity?.(),
      privacy: window.gibbonPrivacy?.updateBusy?.() === true,
    };
  }
  function busy(state) {
    return state.editing || state.saving || state.exporting || state.privacy || window.gibbonUpdateBusy?.() === true;
  }
  function acknowledge(epoch) {
    if (epoch !== frozenEpoch) return;
    cancelDrain();
    const requests = {};
    const now = performance.now();
    for (const { kind, start } of pending.values()) {
      const bucket = requests[kind] ??= { count: 0, oldest_ms: 0 };
      bucket.count++;
      bucket.oldest_ms = Math.max(bucket.oldest_ms, Math.round(now - start));
    }
    const report = {
      ...activity(),
      pending: pending.size,
      requests,
      oldest_ms: pending.size ? Math.round(now - pending.values().next().value.start) : 0,
    };
    const safe = pending.size === 0 && !busy(report);
    raw('updater_ack', { epoch, safe, report }).catch(() => { /* Missing ack blocks installation. */ });
  }
  tauri.event.listen('updater-prepare', ({ payload: epoch }) => {
    if (epoch <= latestEpoch) return;
    latestEpoch = epoch;
    frozenEpoch = epoch;
    cancelDrain();
    if (pending.size && !busy(activity()) && [...pending.values()].every(request => background.has(request.kind))) {
      acknowledgeDrained = () => acknowledge(epoch);
      drainTimer = setTimeout(acknowledgeDrained, drainMs);
    } else {
      acknowledge(epoch);
    }
  }).catch(() => {});
  tauri.event.listen('updater-release', ({ payload: epoch }) => {
    if (epoch < latestEpoch) return;
    latestEpoch = epoch;
    cancelDrain();
    frozenEpoch = null;
  }).catch(() => {});
})();
