import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { themeFor } from '../ui/theme.mjs';

async function setup(storage = new Map()) {
  const elements = new Map();
  const element = id => {
    if (!elements.has(id)) elements.set(id, {
      hidden: false, disabled: false, value: '', dataset: {}, listeners: {}, attributes: {},
      addEventListener(type, listener) { this.listeners[type] = listener; },
      setAttribute(key, value) { this.attributes[key] = value; }, focus() {},
      getBoundingClientRect() { return { width: 298, height: this.hidden ? 0 : 42 }; },
      cloneNode() { return { style: {}, getBoundingClientRect: () => ({ width: 298, height: 120 }), remove() {} }; },
    });
    return elements.get(id);
  };
  const styles = {};
  const events = {};
  const timers = [];
  const calls = [];
  let note = { revision: 1, markdown: '# 원문', html: '<h1>원문</h1>', color: '#20252e' };
  let saveError;
  let warpError;
  let contact = false;
  const context = vm.createContext({
    localStorage: { getItem: key => storage.get(key), setItem: (key, value) => storage.set(key, value) },
    themeFor, clearTimeout() {}, setTimeout(callback) { timers.push(callback); },
    document: {
      body: { appendChild() {}, classList: { toggle() {} } },
      getElementById: element, querySelectorAll: () => [],
      addEventListener(type, listener) { events[type] = listener; },
      documentElement: { style: { setProperty(key, value) { styles[key] = value; } }, addEventListener() {} },
    },
    window: { __TAURI__: {
      event: { async listen() { return () => {}; } },
      core: { async invoke(command, args) {
        calls.push([command, args]);
        if (command === 'warp_note' && warpError) throw warpError;
        if (command === 'poll_warp') { const result = contact; contact = false; return result; }
        if (command === 'snapshot') return { note: { ...note }, always_on_top: true };
        if (command === 'save_note') {
          if (saveError) throw saveError;
          assert.equal(args.expectedRevision, note.revision);
          note = { ...note, markdown: args.markdown, html: '<p>저장됨</p>', revision: note.revision + 1 };
          return note;
        }
      } },
      window: { getCurrentWindow: () => ({ startDragging: async () => calls.push(['drag']) }) },
    } },
  });
  vm.runInContext(readFileSync(new URL('../ui/app.js', import.meta.url), 'utf8').replace("import { themeFor } from './theme.mjs';", ''), context);
  await new Promise(setImmediate);
  return { element, styles, events, calls, refresh: () => timers.find(fn => fn.name === 'refresh')(),
    pollWarp: () => timers.find(fn => fn.name === 'pollWarp')(),
    contact: () => { contact = true; }, failWarp: error => { warpError = error; },
    update: patch => { note = { ...note, ...patch }; }, failSave: error => { saveError = error; } };
}

test('content drag delegates to the backend command that cancels entrance before native dragging', async () => {
  const app = await setup();
  let prevented = false;
  app.events.mousedown({ button: 0, target: { closest: () => false }, preventDefault() { prevented = true; } });
  await new Promise(setImmediate);
  assert.equal(prevented, true);
  assert.equal(app.calls.filter(([name]) => name === 'drag_note').length, 1);
  assert.equal(app.calls.filter(([name]) => name === 'drag').length, 0);
});

test('editor and non-primary clicks never start entrance-cancelling drag', async () => {
  const app = await setup();
  for (const [button, interactive] of [[0, true], [1, false], [2, false]]) {
    app.events.mousedown({ button, target: { closest: () => interactive }, preventDefault() { assert.fail('must preserve native interaction'); } });
  }
  assert.equal(app.calls.filter(([name]) => name === 'drag_note').length, 0);
});
