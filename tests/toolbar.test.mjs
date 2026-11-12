import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {readFileSync} from 'node:fs';

test('toolbar waits for native fold and restores the same state on expansion', async () => {
  const classes = new Set();
  let focused = true;
  const windowEvents = {};
  const elements = new Map();
  const element = id => {
    if (!elements.has(id)) elements.set(id, {
      hidden: false, offsetWidth: 24, clientWidth: 300, scrollWidth: 100, scrollLeft: 0,
      listeners: {}, attributes: {},
      addEventListener(name, callback) { this.listeners[name] = callback; },
      setAttribute(name, value) { this.attributes[name] = value; },
      getAttribute(name) { return this.attributes[name]; },
      removeAttribute(name) { delete this.attributes[name]; },
      querySelectorAll() { return []; },
    });
    return elements.get(id);
  };
  const body = {classList: {
    contains: name => classes.has(name),
    toggle(name, enabled) { if (!enabled) { classes.delete(name); return false; } classes.add(name); return true; },
  }};
  let folded = false;
  const calls = [];
  const content = element('content');
  element('controls').querySelectorAll = () => [element('close'), element('toolbar-toggle')];
  let hit = null;
  const nativeEvents = {};
  const document = {body, elementFromPoint: () => hit, hasFocus: () => focused, getElementById: id => id === 'fold-title' ? null : element(id)};
  const env = {addEventListener(name, callback) { windowEvents[name] = callback; }, __TAURI__: {event: {listen(name, callback) { nativeEvents[name] = callback; }}, core: {invoke: async name => { calls.push(name); folded = !folded; return folded; }}}};
  vm.runInNewContext(readFileSync(new URL('../ui/toolbar.js', import.meta.url), 'utf8'), {
    document, window: env, ResizeObserver: class {observe() {}}, MutationObserver: class {observe() {}},
  });
  const close = element('close');
  close.listeners.pointerenter();
  assert.equal(close.attributes['data-pointer-over'], '');
  close.listeners.pointerleave();
  assert.equal(close.attributes['data-pointer-over'], undefined);
  focused = false;
  close.listeners.pointerenter();
  assert.equal(close.attributes['data-pointer-over'], '');
  close.listeners.pointerleave();
  close.disabled = true;
  close.listeners.pointerenter();
  assert.equal(close.attributes['data-pointer-over'], undefined);
  close.disabled = false;
  focused = true;
  close.listeners.pointerenter();
  // Focus loss is unrelated to pointer exit and must not clear visual hover.
  windowEvents.blur?.();
  assert.equal(close.attributes['data-pointer-over'], '');
  const toggle = element('toolbar-toggle');
  // Native coordinates must work without DOM focus in either toolbar state,
  // including hits on the SVG child, gaps/drag areas, disabled and hidden tools.
  for (const collapsed of [false, true]) {
    env.gibbonToolbarFold(collapsed);
    for (const active of [false, true]) {
      focused = active;
      for (const button of [toggle, close]) {
        hit = {closest: () => button};
        nativeEvents['toolbar-pointer']({payload: {x: 280, y: 10}});
        assert.equal(button.attributes['data-pointer-over'], '');
        assert.equal((button === close ? toggle : close).attributes['data-pointer-over'], undefined);
      }
      hit = {closest: () => null};
      nativeEvents['toolbar-pointer']({payload: {x: 80, y: 10}});
      assert.equal(close.attributes['data-pointer-over'], undefined);
      hit = {closest: () => close};
      close.disabled = true;
      nativeEvents['toolbar-pointer']({payload: {x: 280, y: 10}});
      assert.equal(close.attributes['data-pointer-over'], undefined);
      close.disabled = false;
      nativeEvents['toolbar-pointer']({payload: {x: 280, y: 10}});
      nativeEvents['toolbar-pointer']({payload: null});
      assert.equal(close.attributes['data-pointer-over'], undefined);
    }
  }
  env.gibbonToolbarFold(false);
  await toggle.listeners.click();
  assert.equal(classes.has('toolbar-collapsed'), true);
  assert.equal(toggle.attributes['aria-expanded'], 'false');
  assert.equal(toggle.attributes['aria-label'], '메모 펼치기');
  assert.equal(toggle.title, '메모 펼치기');
  assert.equal(element('toolbar-previous').hidden, true);
  assert.equal(element('toolbar-next').hidden, true);
  assert.equal(calls.at(-1), 'chrome_fold');
  assert.equal(element('close').hidden, false);
  await toggle.listeners.click();
  assert.equal(classes.has('toolbar-collapsed'), false);
  assert.equal(toggle.attributes['aria-expanded'], 'true');
  assert.equal(toggle.title, '메모 접기');
  let delegated = 0;
  env.gibbonToggleNote = async () => { delegated++; env.gibbonToolbarFold(true); };
  await toggle.listeners.click();
  assert.equal(delegated, 1);
  assert.equal(calls.length, 2);
  assert.equal(classes.has('toolbar-collapsed'), true);
  assert.equal(calls.at(-1), 'chrome_fold');
});
