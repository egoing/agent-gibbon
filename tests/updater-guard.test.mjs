import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('../ui/updater-guard.js', import.meta.url), 'utf8');
const tick = () => new Promise(setImmediate);
const idleReport = { editing: false, saving: false, exporting: false, privacy: false, pending: 0, requests: {}, oldest_ms: 0 };
const plain = value => JSON.parse(JSON.stringify(value));

function setup({ busy = () => false, failListen = null } = {}) {
  const listeners = new Map(), domListeners = new Map(), requests = [];
  const timers = [];
  let handleInvoke = async () => ({ ok: true });
  const window = {
    gibbonUpdateBusy: busy,
    gibbonUpdateActivity: () => ({ editing: busy() }),
    addEventListener(name, callback, capture) {
      const entries = domListeners.get(name) || [];
      entries.push({ callback, capture }); domListeners.set(name, entries);
    },
  };
  const event = {
    listen(name, callback) {
      if (name === failListen) return Promise.reject(new Error('listener unavailable'));
      listeners.set(name, callback);
      return Promise.resolve(() => listeners.delete(name));
    },
  };
  const core = { invoke: async (command, args) => { requests.push({ command, args }); return handleInvoke(command, args); } };
  window.__TAURI__ = { core, event };
  const context = vm.createContext({ window, Promise, setTimeout: callback => { timers.push(callback); return timers.length; }, performance: { now: () => 100 } });
  vm.runInContext(source, context);
  return { window, listeners, domListeners, requests, fire: (name, payload) => listeners.get(name)?.({ payload }), runTimers: () => timers.splice(0).forEach(callback => callback()), setInvoke: fn => { handleInvoke = fn; }, context };
}

test('prepare acknowledges safe=false while a write command is pending', async () => {
  const held = setup();
  let resolveCommand, started = false;
  held.setInvoke(command => {
    if (command === 'save_note') return new Promise(resolve => { started = true; resolveCommand = resolve; });
    return { ok: true };
  });
  const command = held.window.gibbonInvoke('save_note');
  await tick();
  assert.equal(started, true);
  held.fire('updater-prepare', 'epoch-pending');
  await tick();
  assert.deepEqual(plain(held.requests.at(-1)), { command: 'updater_ack', args: { epoch: 'epoch-pending', safe: false, report: { ...idleReport, pending: 1, requests: { save_note: { count: 1, oldest_ms: 0 } } } } });
  resolveCommand({ ok: true }); await command;
});

test('prepare freezes input, drains a short in-flight read, then acknowledges safe', async () => {
  const app = setup();
  let resolveRead;
  app.setInvoke(command => command === 'manager_notes' ? new Promise(resolve => { resolveRead = resolve; }) : {});
  const read = app.window.gibbonInvoke('manager_notes');
  await tick();
  const preparing = app.fire('updater-prepare', 'drain-read');
  await tick();
  assert.equal(app.requests.some(request => request.command === 'updater_ack'), false);
  resolveRead({ ok: true });
  await Promise.all([read, preparing]);
  assert.equal(app.requests.at(-1).args.safe, true);
  assert.equal(app.requests.at(-1).args.report.pending, 0);
});

test('prepare drains a hidden-window snapshot and both native polling requests together', async () => {
  const app = setup();
  const pending = new Map();
  app.setInvoke(command => ['snapshot', 'poll_fold', 'poll_warp'].includes(command)
    ? new Promise(resolve => pending.set(command, resolve)) : {});
  const calls = ['snapshot', 'poll_fold', 'poll_warp'].map(command => app.window.gibbonInvoke(command));
  await tick();
  const preparing = app.fire('updater-prepare', 'hidden-window-reads');
  await tick();
  assert.equal(app.requests.some(request => request.command === 'updater_ack'), false);
  for (const resolve of pending.values()) resolve({ ok: true });
  await Promise.all([...calls, preparing]);
  assert.equal(app.requests.at(-1).args.safe, true);
  assert.equal(app.requests.at(-1).args.report.pending, 0);
});

test('a stalled in-flight read reaches the bounded drain and refuses installation', async () => {
  const app = setup();
  app.setInvoke(command => command === 'snapshot' ? new Promise(() => {}) : {});
  app.window.gibbonInvoke('snapshot');
  await tick();
  const preparing = app.fire('updater-prepare', 'stalled-read');
  await tick();
  assert.equal(app.requests.some(request => request.command === 'updater_ack'), false);
  app.runTimers();
  await preparing;
  assert.equal(app.requests.at(-1).args.safe, false);
  assert.deepEqual(plain(app.requests.at(-1).args.report.requests), { snapshot: { count: 1, oldest_ms: 0 } });
});

test('a pending write refuses immediately without waiting for the read drain', async () => {
  const app = setup();
  app.setInvoke(command => command === 'save_note' ? new Promise(() => {}) : {});
  app.window.gibbonInvoke('save_note');
  await tick();
  const preparing = app.fire('updater-prepare', 'write-pending');
  await preparing;
  assert.equal(app.requests.at(-1).args.safe, false);
  assert.equal(app.requests.at(-1).args.report.pending, 1);
});

test('prepare acknowledges safe=false while an editing callback reports busy', async () => {
  const app = setup({ busy: () => true });
  app.fire('updater-prepare', 17); await tick();
  assert.deepEqual(plain(app.requests), [{ command: 'updater_ack', args: { epoch: 17, safe: false, report: { ...idleReport, editing: true } } }]);
});

test('prepare acknowledges safe=true when no command or editing work is active', async () => {
  const app = setup();
  app.fire('updater-prepare', 'idle-epoch'); await tick();
  assert.deepEqual(plain(app.requests), [{ command: 'updater_ack', args: { epoch: 'idle-epoch', safe: true, report: idleReport } }]);
});

test('frozen state rejects new commands and captures input until release', async () => {
  const app = setup();
  app.fire('updater-prepare', 'freeze'); await tick();
  await assert.rejects(app.window.gibbonInvoke('manager_notes'), /업데이트 설치를 준비하고 있습니다/);
  const inputNames = ['pointerdown', 'keydown', 'beforeinput', 'click', 'drop'];
  for (const name of inputNames) {
    let prevented = 0, stopped = 0;
    app.domListeners.get(name)[0].callback({ preventDefault() { prevented++; }, stopImmediatePropagation() { stopped++; } });
    assert.equal(prevented, 1, `${name} prevented while frozen`);
    assert.equal(stopped, 1, `${name} stopped while frozen`);
  }
  app.fire('updater-release');
  await app.window.gibbonInvoke('manager_notes');
  assert.equal(app.requests.at(-1).command, 'manager_notes');
  let prevented = 0, stopped = 0;
  app.domListeners.get('click')[0].callback({ preventDefault() { prevented++; }, stopImmediatePropagation() { stopped++; } });
  assert.equal(prevented, 0); assert.equal(stopped, 0);
});

test('updater status and install commands do not count as pending manager work', async () => {
  const app = setup();
  let resolveStatus, resolveInstall;
  app.setInvoke(command => {
    if (command === 'updater_status') return new Promise(resolve => { resolveStatus = resolve; });
    if (command === 'updater_install') return new Promise(resolve => { resolveInstall = resolve; });
    return { ok: true };
  });
  const status = app.window.gibbonInvoke('updater_status');
  await tick();
  app.fire('updater-prepare', 'status-only'); await tick();
  assert.deepEqual(plain(app.requests.at(-1)), { command: 'updater_ack', args: { epoch: 'status-only', safe: true, report: idleReport } });
  const install = app.window.gibbonInvoke('updater_install');
  await tick();
  app.fire('updater-prepare', 'install-only'); await tick();
  assert.deepEqual(plain(app.requests.at(-1)), { command: 'updater_ack', args: { epoch: 'install-only', safe: true, report: idleReport } });
  resolveStatus({ phase: 'idle' }); resolveInstall({ phase: 'ready' });
  await Promise.all([status, install]);
});

test('a failed prepare listener registration sends no acknowledgement', async () => {
  const app = setup({ failListen: 'updater-prepare' });
  await tick();
  assert.equal(app.listeners.has('updater-prepare'), false);
  assert.equal(app.requests.some(request => request.command === 'updater_ack'), false);
});

test('a failed acknowledgement invocation is contained and cannot report success', async () => {
  const app = setup();
  app.setInvoke(command => command === 'updater_ack' ? Promise.reject(new Error('ack failed')) : {});
  await app.fire('updater-prepare', 'ack-failure');
  assert.equal(app.requests.at(-1).command, 'updater_ack');
});

test('unknown command names and request arguments are absent from the installation report', async () => {
  const app = setup();
  let finish;
  app.setInvoke(command => command === 'private-title'
    ? new Promise(resolve => { finish = resolve; }) : {});
  const pending = app.window.gibbonInvoke('private-title', { markdown: 'private-body' });
  await tick();
  app.fire('updater-prepare', 2);
  await tick();
  const report = plain(app.requests.at(-1).args.report);
  assert.deepEqual(report.requests, { other: { count: 1, oldest_ms: 0 } });
  assert.doesNotMatch(JSON.stringify(report), /private-title|private-body/);
  finish();
  await pending;
  app.fire('updater-release');
  app.fire('updater-prepare', 3);
  await tick();
  assert.equal(app.requests.at(-1).args.safe, true);
});
