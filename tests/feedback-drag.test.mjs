import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {readFileSync} from 'node:fs';

test('feedback text and blank space drag the owner while controls keep their actions', async () => {
  const calls = [];
  const listeners = {};
  const surface = {
    style: {}, dataset: {}, classList: {toggle() {}},
    addEventListener(name, callback) {listeners[name] = callback;},
    getBoundingClientRect() {return {width: 200, height: 30};},
  };
  const message = {setAttribute() {}};
  const actions = {children: []};
  const invoke = async (name) => {
    calls.push(name);
    if (name === 'feedback_snapshot') return {max_width: 300, view: {message: '안내', buttons: [], revision: null}};
    if (name === 'layout_feedback') return {edge: 'below'};
  };
  vm.runInNewContext(readFileSync(new URL('../ui/feedback.js', import.meta.url), 'utf8').replace(/^import .*;$/gm, ''), {
    window: {__TAURI__: {core: {invoke}}},
    document: {
      getElementById: id => ({feedback: surface, message, actions})[id],
      documentElement: {style: {setProperty() {}}}, body: {style: {}}, addEventListener() {},
    },
    memoThemeFor: () => ({background: '#fff', foreground: '#222'}), readMemoAppearance: () => 'white',
    setTimeout() {}, console,
  });
  await new Promise(setImmediate);
  let prevented = 0;
  const event = (button, control) => ({button, target: {closest: () => control}, preventDefault() {prevented++;}});
  listeners.mousedown(event(1, null));
  listeners.mousedown(event(0, {}));
  assert.equal(calls.filter(name => name === 'drag_feedback').length, 0);
  listeners.mousedown(event(0, null));
  assert.equal(calls.filter(name => name === 'drag_feedback').length, 1);
  assert.equal(prevented, 1);
});
