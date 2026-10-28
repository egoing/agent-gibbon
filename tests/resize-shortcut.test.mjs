import test from 'node:test';
import assert from 'node:assert/strict';
import { wheelRequest, resizeTip } from '../ui/resize-shortcut.mjs';
test('wheel modifiers select independent axes and normalize units', () => {
  assert.deepEqual(wheelRequest({ metaKey: true, deltaY: -20 }), { axis: 'both', delta: -20 });
  assert.deepEqual(wheelRequest({ ctrlKey: true, deltaY: 3, deltaMode: 1 }), { axis: 'horizontal', delta: 48 });
  assert.deepEqual(wheelRequest({ altKey: true, deltaY: 1, deltaMode: 2 }), { axis: 'vertical', delta: 240 });
});
test('ordinary scroll, mixed modifiers, invalid and horizontal-only input are ignored', () => {
  for (const event of [{ deltaY: 10 }, { metaKey: true, ctrlKey: true, deltaY: 10 }, { altKey: true, shiftKey: true, deltaY: 10 }, { ctrlKey: true, deltaY: NaN }, { metaKey: true, deltaY: 0, deltaX: 10 }]) assert.equal(wheelRequest(event), null);
});
test('tips identify the platform keys', () => {
  assert.match(resizeTip(true), /⌘.*Ctrl.*Option/);
  assert.match(resizeTip(false), /Win.*Ctrl.*Alt/);
});
