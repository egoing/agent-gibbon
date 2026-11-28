import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { readFileSync, readdirSync } from 'node:fs';

const read = path => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8');
const capabilities = readdirSync(new URL('../src-tauri/capabilities/', import.meta.url))
  .filter(name => name.endsWith('.json')).map(name => JSON.parse(read(`src-tauri/capabilities/${name}`)));
const labels = ['direction-guide', 'layout-undo', 'memo-backdrop', 'memo-overview'];
const tick = () => new Promise(setImmediate);

// Load the scripts actually shipped by each page, with its actual event grants.
// This catches missing wiring that guard-only tests cannot exercise.
function mount(label) {
  const listeners = new Map(), dom = new Map(), requests = [];
  let invoke = async () => {};
  const button = { classList: { add() {}, remove() {} } };
  const status = {};
  const controls = { querySelectorAll: () => [], addEventListener: (name, fn) => dom.set(name, fn) };
  const document = {
    querySelector: selector => selector === 'nav' ? controls : selector === 'button' ? button : status,
    querySelectorAll: () => [],
    documentElement: { setAttribute() {}, hasAttribute: () => true },
    addEventListener: (name, fn) => dom.set(name, fn),
  };
  const window = {
    overviewGeneration: 1,
    addEventListener: (name, fn) => dom.set(`capture:${name}`, fn),
    __TAURI__: {
      core: { invoke: async (command, args) => { requests.push({ command, args }); return invoke(command, args); } },
      event: { listen: async (name, fn) => {
        const allowed = capabilities.some(c => c.windows?.includes(label) && c.permissions.includes('core:event:allow-listen'));
        if (!allowed) throw new Error('event permission denied');
        listeners.set(name, fn);
      } },
      webviewWindow: { getCurrentWebviewWindow: () => ({ listen: async () => {} }) },
    },
  };
  const context = vm.createContext({ window, document, console, performance, Promise, setTimeout, clearTimeout });
  for (const match of read(`ui/${label}.html`).matchAll(/<script\b[^>]*src="([^"]+)"[^>]*><\/script>/g)) {
    vm.runInContext(read(`ui/${match[1]}`), context, { filename: match[1] });
  }
  const fire = (name, epoch) => {
    assert.equal(typeof listeners.get(name), 'function', `${label} must receive ${name}`);
    listeners.get(name)({ payload: epoch });
  };
  const action = () => label === 'layout-undo' ? button.onclick()
    : label === 'memo-backdrop' ? dom.get('pointerdown')({ button: 0 })
    : dom.get('click')({ target: { closest: () => ({ dataset: { action: 'arrange' } }) } });
  return { window, requests, dom, fire, action, setInvoke: fn => { invoke = fn; } };
}

for (const label of labels) {
  test(`${label} acknowledges prepare, freezes input and releases for retry`, async () => {
    const app = mount(label);
    await tick();
    app.fire('updater-prepare', 1);
    await tick();
    const ack = app.requests.find(r => r.command === 'updater_ack');
    assert.equal(ack?.args.safe, true);
    assert.equal(ack.args.report.pending, 0);
    await assert.rejects(app.window.jajanInvoke('snapshot'), /업데이트 설치/);
    let stopped = false;
    app.dom.get('capture:click')({ preventDefault() {}, stopImmediatePropagation() { stopped = true; } });
    assert.equal(stopped, true);
    app.fire('updater-release', 1);
    await app.window.jajanInvoke('snapshot');
    app.fire('updater-prepare', 2);
    assert.equal(app.requests.at(-1).args.epoch, 2);
    assert.equal(app.requests.at(-1).args.safe, true);
    app.fire('updater-release', 2);
  });
}

for (const label of labels.filter(label => label !== 'direction-guide')) {
  test(`${label} includes its real layout action in the safety handshake`, async () => {
    const app = mount(label);
    let finish;
    app.setInvoke(command => command === 'updater_ack' ? undefined : new Promise(resolve => { finish = resolve; }));
    const action = app.action();
    await tick();
    app.fire('updater-prepare', 1);
    const ack = app.requests.at(-1);
    assert.equal(ack.command, 'updater_ack');
    assert.equal(ack.args.safe, false, 'pending layout mutation must block installation');
    assert.equal(ack.args.report.pending, 1);
    finish();
    await action;
    await tick();
    app.fire('updater-release', 1);
    app.fire('updater-prepare', 2);
    assert.equal(app.requests.at(-1).args.safe, true);
    app.fire('updater-release', 2);
  });
}
