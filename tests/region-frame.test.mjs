import { sourceLanguageWindow } from './fixtures/language.mjs';
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';

const source = readFileSync(new URL('../ui/region-frame.js', import.meta.url), 'utf8');

for (const condition of ['restored', 'onboarding']) {
  test('ordinary hidden page keeps initialization and never reveals during ' + condition, async () => {
    const commands = [];
    const window = {__TAURI__: {core: {async invoke(command) {
      commands.push(command);
      if (condition === 'onboarding') throw new Error('onboarding_required');
      return false;
    }}}};
    vm.runInNewContext(source, sourceLanguageWindow({window}));
    let initialized = false;
    await window.jajanRegionFrame.prepare(() => { initialized = true; });
    await window.jajanRegionFrame.ready();
    assert.equal(initialized, true);
    assert.deepEqual(commands, ['region_frame_placement']);
  });
}

test('a rejected Welcome task presents its error after all initial tasks settle', async () => {
  const order = [];
  const status = {textContent: ''};
  const document = {
    getElementById: id => { assert.equal(id, 'status'); return status; },
    body: {getBoundingClientRect: () => order.push('layout')},
  };
  const window = {__TAURI__: {core: {async invoke(command) {
    if (command === 'region_frame_placement') return true;
    assert.equal(command, 'region_frame_ready');
    assert.equal(status.textContent, 'Error: Welcome 조회 실패');
    order.push('show');
  }}}};
  vm.runInNewContext(source, sourceLanguageWindow({window, document}));
  const frame = window.jajanRegionFrame;
  await frame.prepare(async () => { throw new Error('Welcome 조회 실패'); });
  let applyTheme;
  frame.prepare(() => new Promise(resolve => { applyTheme = resolve; }));
  const showing = frame.ready();
  await Promise.resolve();
  assert.deepEqual(order, []);
  applyTheme();
  await showing;
  await frame.ready();
  assert.deepEqual(order, ['layout', 'show']);
});

test('a newly drawn surface stays transparent until its themed paint is prepared', async () => {
  const style = {removeProperty(key) { delete this[key]; }};
  const document = {
    documentElement: {style},
    body: {getBoundingClientRect() { assert.equal(style.opacity, undefined); }},
  };
  const window = {jajanPreparingRegion: true, __TAURI__: {core: {async invoke(command) {
    if (command === 'region_frame_placement') return true;
    assert.equal(command, 'region_frame_ready');
    assert.equal(style.opacity, undefined);
  }}}};
  vm.runInNewContext(source, sourceLanguageWindow({window, document}));
  assert.equal(style.opacity, '0');
  await window.jajanRegionFrame.placement;
  assert.equal(style.opacity, '0');
  await window.jajanRegionFrame.ready();
});
