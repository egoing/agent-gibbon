import { sourceLanguageWindow } from './fixtures/language.mjs';
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';

function mount() {
  const requests = [];
  const status = {};
  let handler;
  let onPointer;
  const buttons = [0, 1].map(() => ({ toggleAttribute(_, value) { this.highlighted = value; }, closest() { return this; } }));
  const controls = { querySelectorAll: () => buttons, addEventListener(type, fn) { assert.equal(type, 'click'); handler = fn; } };
  const window = { __TAURI__: { webviewWindow: { getCurrentWebviewWindow: () => ({ listen(_, callback) { onPointer = callback; } }) }, core: { invoke(command, args) {
    return new Promise((resolve, reject) => requests.push({ command, ...args, resolve, reject }));
  } } } };
  window.jajanInvoke = window.__TAURI__.core.invoke;
  const document = { elementFromPoint: x => buttons[x], querySelector: s => s === 'nav' ? controls : status };
  vm.runInNewContext(readFileSync(new URL('../ui/memo-overview.js', import.meta.url), 'utf8'), sourceLanguageWindow({ window, document }));
  return { buttons, pointer: payload => onPointer({ payload }), enter: window.beginOverview, requests, status,
    click: action => handler({ target: { closest: () => ({ dataset: { action } }) } }) };
}

test('entry leaves frames alone and each explicit button dispatches once', async () => {
  const ui = mount();
  ui.enter(1);
  assert.equal(ui.requests.length, 0);
  const first = ui.click('arrange');
  await ui.click('arrange');
  assert.equal(ui.requests.length, 1);
  assert.deepEqual([ui.requests[0].command, ui.requests[0].action, ui.requests[0].generation],
    ['overview_action', 'arrange', 1]);
  ui.requests[0].resolve();
  await first;
  const close = ui.click('close');
  assert.equal(ui.requests[1].action, 'close');
  ui.requests[1].resolve();
  await close;
});

test('late results from a previous visit cannot unlock controls or replace new feedback', async () => {
  const ui = mount();
  ui.enter(4);
  const old = ui.click('close');
  ui.enter(5);
  const current = ui.click('arrange');
  ui.requests[0].reject('이전 요청');
  await old;
  assert.equal(ui.status.hidden, true);
  await ui.click('close');
  assert.equal(ui.requests.length, 2);
  ui.requests[1].reject('배치할 공간이 부족합니다');
  await current;
  assert.equal(ui.status.textContent, '배치할 공간이 부족합니다');
  const retry = ui.click('close');
  assert.equal(ui.requests[2].generation, 5);
  ui.requests[2].resolve();
  await retry;
});

test('inactive overview highlights only the button under the pointer without a click', () => {
  const ui = mount();
  ui.enter(1);
  ui.pointer({ x: 1, y: 0 });
  assert.deepEqual(ui.buttons.map(b => b.highlighted), [false, true]);
  ui.pointer({ x: 0, y: 0 });
  assert.deepEqual(ui.buttons.map(b => b.highlighted), [true, false]);
  ui.pointer(null);
  assert.deepEqual(ui.buttons.map(b => b.highlighted), [false, false]);
  assert.equal(ui.requests.length, 0);
});
