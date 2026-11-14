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
  const context = vm.createContext({ window, Promise, setTimeout, clearTimeout, performance: { now: () => 100 } });
  vm.runInContext(source, context);
  return { window, listeners, domListeners, requests, fire: (name, payload) => listeners.get(name)?.({ payload }), setInvoke: fn => { handleInvoke = fn; }, context };
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
  held.fire('updater-prepare', 1);
  await tick();
  assert.deepEqual(plain(held.requests.at(-1)), { command: 'updater_ack', args: { epoch: 1, safe: false, report: { ...idleReport, pending: 1, requests: { save_note: { count: 1, oldest_ms: 0 } } } } });
  resolveCommand({ ok: true }); await command;
});

test('prepare acknowledges safe=false while an editing callback reports busy', async () => {
  const app = setup({ busy: () => true });
  app.fire('updater-prepare', 17); await tick();
  assert.deepEqual(plain(app.requests), [{ command: 'updater_ack', args: { epoch: 17, safe: false, report: { ...idleReport, editing: true } } }]);
});

test('prepare acknowledges safe=true when no command or editing work is active', async () => {
  const app = setup();
  app.fire('updater-prepare', 2); await tick();
  assert.deepEqual(plain(app.requests), [{ command: 'updater_ack', args: { epoch: 2, safe: true, report: idleReport } }]);
});

test('frozen state rejects new commands and captures input until release', async () => {
  const app = setup();
  app.fire('updater-prepare', 3); await tick();
  await assert.rejects(app.window.gibbonInvoke('manager_notes'), /업데이트 설치를 준비하고 있습니다/);
  const inputNames = ['pointerdown', 'keydown', 'beforeinput', 'click', 'drop'];
  for (const name of inputNames) {
    let prevented = 0, stopped = 0;
    app.domListeners.get(name)[0].callback({ preventDefault() { prevented++; }, stopImmediatePropagation() { stopped++; } });
    assert.equal(prevented, 1, `${name} prevented while frozen`);
    assert.equal(stopped, 1, `${name} stopped while frozen`);
  }
  app.fire('updater-release', 3);
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
  app.fire('updater-prepare', 4); await tick();
  assert.deepEqual(plain(app.requests.at(-1)), { command: 'updater_ack', args: { epoch: 4, safe: true, report: idleReport } });
  const install = app.window.gibbonInvoke('updater_install');
  await tick();
  app.fire('updater-prepare', 5); await tick();
  assert.deepEqual(plain(app.requests.at(-1)), { command: 'updater_ack', args: { epoch: 5, safe: true, report: idleReport } });
  resolveStatus({ phase: 'idle' }); resolveInstall({ phase: 'ready' });
  await Promise.all([status, install]);
});

test('a failed prepare listener registration sends no acknowledgement', async () => {
  const app = setup({ failListen: 'updater-prepare' });
  await tick();
  assert.equal(app.listeners.has('updater-prepare'), false);
  assert.equal(app.requests.some(request => request.command === 'updater_ack'), false);
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
  app.fire('updater-release', 2);
  app.fire('updater-prepare', 3);
  await tick();
  assert.equal(app.requests.at(-1).args.safe, true);
});

function heldRequests(commands) {
  const app = setup();
  const completions = new Map();
  app.setInvoke(command => commands.includes(command)
    ? new Promise(resolve => completions.set(command, resolve)) : {});
  const work = commands.map(command => app.window.gibbonInvoke(command));
  const acks = () => app.requests.filter(request => request.command === 'updater_ack').map(request => request.args);
  return { app, completions, work, acks };
}

test('background polls settle before acknowledgement, with new work frozen', async () => {
  const commands = ['poll_warp', 'poll_fold', 'snapshot', 'manager_notes', 'terminal_info'];
  const { app, completions, work, acks } = heldRequests(commands);
  await tick();
  app.fire('updater-prepare', 1);
  await assert.rejects(app.window.gibbonInvoke('save_note'), /업데이트 설치/);
  assert.equal(acks().length, 0);
  for (const complete of completions.values()) complete({});
  await Promise.all(work);
  await new Promise(resolve => setTimeout(resolve, 5));
  assert.deepEqual(plain(acks()), [{ epoch: 1, safe: true, report: idleReport }]);
});

test('a held background poll stays unsafe after its drain deadline', async () => {
  const { app, completions, work, acks } = heldRequests(['poll_fold']);
  await tick();
  app.fire('updater-prepare', 1);
  await new Promise(resolve => setTimeout(resolve, 550));
  assert.equal(acks().length, 1);
  assert.equal(acks()[0].safe, false);
  assert.equal(acks()[0].report.requests.poll_fold.count, 1);
  completions.get('poll_fold')({});
  await Promise.all(work);
  await tick();
  assert.equal(acks().length, 1);
});

test('a save alongside background polls refuses immediately', async () => {
  const { app, completions, work, acks } = heldRequests(['poll_warp', 'save_note']);
  await tick();
  app.fire('updater-prepare', 1);
  assert.equal(acks()[0].safe, false);
  assert.equal(acks()[0].report.requests.save_note.count, 1);
  for (const complete of completions.values()) complete({});
  await Promise.all(work);
});

test('a released epoch cannot acknowledge later work or unfreeze a newer epoch', async () => {
  const { app, completions, work, acks } = heldRequests(['snapshot']);
  await tick();
  app.fire('updater-prepare', 1);
  app.fire('updater-release', 1);
  app.fire('updater-prepare', 2);
  app.fire('updater-release', 1);
  app.fire('updater-prepare', 1);
  await assert.rejects(app.window.gibbonInvoke('save_note'), /업데이트 설치/);
  app.window.gibbonUpdateActivity = () => ({ exporting: true });
  completions.get('snapshot')({});
  await Promise.all(work);
  await new Promise(resolve => setTimeout(resolve, 5));
  assert.deepEqual(plain(acks()), [{ epoch: 2, safe: false, report: { ...idleReport, exporting: true } }]);
});

test('a rejected acknowledgement does not escape the guard', async () => {
  const app = setup();
  app.setInvoke(() => Promise.reject(new Error('ack failed')));
  app.fire('updater-prepare', 1);
  await tick();
  assert.equal(app.requests.at(-1).command, 'updater_ack');
});
