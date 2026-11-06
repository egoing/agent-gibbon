'use strict';
// Explicit local opt-in only. Never routed to Sentry or the heartbeat queue.
(() => {
  const raw = window.__TAURI__?.core?.invoke;
  if (!raw) return;
  let enabled = false, refreshAfter = 0;
  const operations = new Set(['note.refresh', 'note.save', 'terminal.start']);
  const commands = new Set(['snapshot', 'save_note', 'fit_note', 'terminal_start']);
  const stages = new Set(['dom', 'measure', 'fit', 'frame_ready', 'response']);
  const invoke = (command, args) => (window.gibbonInvoke || raw)(command, args);
  const empty = {invoke, stage: () => {}, measure: (_stage, work) => work()};
  window.gibbonLocalFlow = async (operation, work) => {
    const now = window.performance.now();
    if (!enabled || !operations.has(operation) || window.document.visibilityState !== 'visible'
        || (operation === 'note.refresh' && now < refreshAfter)) return work(empty);
    if (operation === 'note.refresh') refreshAfter = now + 2000;
    let flow;
    try { flow = await raw('local_flow_begin', {operation}); } catch { /* optional evidence */ }
    if (!Number.isSafeInteger(flow) || flow <= 0) return work(empty);
    const send = (stage, durationMs) => raw('local_flow_stage', {flow, stage, durationMs}).catch(() => {});
    let framePending = false, finished = false, failed = false;
    const following = [];
    const complete = () => { if (finished && !framePending) void send(failed ? 'failed' : 'complete'); };
    const context = {
      invoke: (command, args) => invoke(command, commands.has(command) ? {...args, localFlow: flow} : args),
      stage: stage => { if (stages.has(stage)) void send(stage); },
      measure: (stage, work) => {
        if (!stages.has(stage)) return work();
        const start = window.performance.now();
        try { return work(); } finally { void send(stage, window.performance.now() - start); }
      },
      follow: promise => { following.push(promise); },
      frame: () => {
        if (framePending) return;
        framePending = true;
        Promise.allSettled(following).then(() => {
          const start = window.performance.now();
          window.requestAnimationFrame(() => window.requestAnimationFrame(() => {
            void send('frame_ready', window.performance.now() - start); framePending = false; complete();
          }));
        });
      },
    };
    try { return await work(context); }
    catch (error) { failed = true; throw error; }
    finally { finished = true; complete(); }
  };
  // A local-only opt-in is independent of consent and never initializes the SDK.
  raw('local_flow_status').then(value => {enabled = value === true;}).catch(() => {});
})();
