'use strict';
// Only fixed operation names + bounded numbers cross IPC. Never read text, keys,
// command arguments, DOM content, URLs, clipboard, console or terminal output.
(() => {
  const core = window.__TAURI__?.core;
  if (!core) return;
  const raw = core.invoke.bind(core);
  const sources = new Set(['app.js','telemetry.js','terminal.js','terminal-ime.js','toolbar.js','manager.js','feedback.js','capture.js','stash.js']);
  for (const kind of ['error', 'unhandledrejection']) {
    window.addEventListener(kind, event => {
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
  let active = false;
  const queue = new Map();
  const now = () => window.performance.now();
  // Install synchronously: app modules may capture invoke before status resolves.
  window.gibbonInvoke = (command, ...args) => {
    const op = operations.get(command);
    if (!active || !op) return raw(command, ...args);
    const started = now();
    return raw(command, ...args).then(value => { record(op, now() - started); return value; },
      error => { record(op, now() - started, true); throw error; });
  };
  function record(op, ms, failed = false) {
      if (!Number.isFinite(ms) || ms < 0 || ms > 600000) return;
      const item = queue.get(op) || { op, duration_ms: 0, count: 0, failure_count: 0 };
      // Max duration per batch exposes latency; count exposes throughput, no raw payload.
      item.duration_ms = Math.max(item.duration_ms, ms);
      if (item.count < 10000) {item.count++;if (failed) item.failure_count++;}
      if (queue.size < 32 || queue.has(op)) queue.set(op, item);
    }
  raw('telemetry_status').then(config => {
    if (!config.detailed) return;
    active = true;
    const stages = new Set(['terminal.ui.ready', 'terminal.first_output', 'terminal.output.parse']);
    // This API accepts fixed English stage names and numeric durations only.
    // It does not exist in the ordinary, non-diagnostic mode.
    window.gibbonMeasure = (op, duration) => {
      if (stages.has(op) && window.document.visibilityState === 'visible') record(op, duration);
    };
    let inFlight = false;
    function beat() {
      if (inFlight) return;
      inFlight = true;
      const measurements = [...queue.values()];
      queue.clear();
      raw('telemetry_heartbeat', { measurements }).catch(() => {}).finally(() => { inFlight = false; });
    }
    const Observer = window.PerformanceObserver;
    for (const [type, op] of [['longtask', 'webview.long_task'], ['long-animation-frame', 'webview.long_frame']]) {
      if (!Observer?.supportedEntryTypes?.includes(type)) continue;
      try { new Observer(list => { for (const entry of list.getEntries()) record(op, entry.duration); }).observe({ type, buffered: false }); } catch { /* WebView capability differs by OS. */ }
    }
    let last = now();
    window.setInterval(() => {
      const time = now(), elapsed = time - last; last = time;
      // Background timers and machine sleep are not evidence of a UI freeze.
      if (window.document.visibilityState === 'visible' && elapsed < 60000 && elapsed > 1250) record('webview.event_loop_lag', elapsed - 1000);
    }, 1000);
    let interactionPending = false;
    window.addEventListener('click', () => {
      if (interactionPending || window.document.visibilityState !== 'visible') return;
      interactionPending = true;
      const start = now();
      window.requestAnimationFrame(() => window.requestAnimationFrame(() => {
        interactionPending = false;
        if (window.document.visibilityState === 'visible') record('webview.interaction', now() - start);
      }));
    }, { passive: true });
    window.setInterval(beat, 2000);
    beat();
  }).catch(() => {});
})();
