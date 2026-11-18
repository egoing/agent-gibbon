import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';

const source = readFileSync(new URL('../ui/web-navigation-keys.js', import.meta.url), 'utf8');
function key(platform, modifiers, accept = true) {
  let handle, result, consumed = false;
  const context = { navigator: {platform}, window: {addEventListener: (_, fn) => { handle = fn; }} };
  vm.runInNewContext(source, context);
  context.installWebNavigationKeys(action => { result = action; return accept; });
  handle({ isTrusted: true, composedPath: () => [], preventDefault() { consumed = true; },
    stopImmediatePropagation() {}, ...modifiers });
  return [result, consumed];
}
test('web navigation follows platform modifiers and preserves editing and Welcome', () => {
  assert.deepEqual(key('MacIntel', {key:'[', metaKey:true}), ['back', true]);
  assert.deepEqual(key('MacIntel', {key:']', metaKey:true}), ['forward', true]);
  assert.deepEqual(key('Win32', {key:'r', ctrlKey:true}), ['reload', true]);
  assert.deepEqual(key('Win32', {key:'ArrowLeft', altKey:true}), ['back', true]);
  assert.deepEqual(key('MacIntel', {key:'ArrowLeft', altKey:true, composedPath:()=>[{tagName:'INPUT'}]}), [undefined, false]);
  assert.deepEqual(key('MacIntel', {key:'F5'}, false), ['reload', false]);
  assert.deepEqual(key('MacIntel', {key:'r', metaKey:true, isTrusted:false}), [undefined, false]);
});
